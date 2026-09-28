//! 全项目npm原生审计任务；执行阶段不争抢同步锁，汇总后串行同步。
use serde_json::{Value, json};
use std::{collections::BTreeMap, path::Path, time::Instant};

/// 在任务执行前验证显式npm参数，不自动继承凭据或安装工具。
pub(crate) fn validate_options(options: &BTreeMap<String, String>) -> Result<(), String> {
    #[cfg(unix)]
    {
        let mut argv = vec!["typescript".into(), ".".into()];
        for (key, value) in options {
            argv.extend([key.clone(), value.clone()]);
        }
        crate::npm_audit_arguments::NpmAuditArguments::parse(&argv)
            .map(|_| ())
            .map_err(str::to_owned)
    }
    #[cfg(not(unix))]
    {
        let _ = options;
        Err("npm原生执行尚不支持此平台".into())
    }
}

/// 使用冻结发现清单与同一原生审计服务，返回待同步观察。
pub(crate) fn run(
    root: &Path,
    build: &str,
    manifest_sha256: Option<&str>,
    options: &BTreeMap<String, String>,
    deadline: Instant,
) -> Value {
    let mut result = json!({"build_root":build,"feedback":null,"observation":null,"backlog_status":"not_connected"});
    #[cfg(unix)]
    {
        use sha2::{Digest, Sha256};
        let manifest = root.join(build).join("package.json");
        if manifest_sha256.is_some_and(|expected| {
            !codeguard_runtime::read_bounded_regular_file(&manifest, 256 * 1024)
                .ok()
                .is_some_and(|bytes| format!("{:x}", Sha256::digest(bytes)) == expected)
        }) || (manifest_sha256.is_none()
            && crate::npm_input_state::observe(&manifest, 256 * 1024).0 == "present")
        {
            result["feedback"] =
                crate::npm_audit_command::feedback("npm_manifest_input_unavailable_or_changed");
            return result;
        }
        let mut argv = vec![
            "typescript".into(),
            root.join(build).to_string_lossy().into_owned(),
            "--workspace".into(),
            root.to_string_lossy().into_owned(),
        ];
        // 历史画像只定位不可用输入；显式原生参数也不能授权该历史根启动工具。
        if manifest_sha256.is_some() {
            for (key, value) in options {
                argv.extend([key.clone(), value.clone()]);
            }
        }
        let args = match crate::npm_audit_arguments::NpmAuditArguments::parse(&argv) {
            Ok(args) => args,
            Err(_) => {
                result["feedback"] =
                    crate::npm_audit_command::feedback("npm_check_options_invalid");
                return result;
            }
        };
        let scope = match crate::npm_workspace_scope::NpmWorkspaceScope::resolve(&args) {
            Ok(scope) => scope,
            Err(reason) => {
                result["feedback"] = crate::npm_audit_command::feedback(reason);
                return result;
            }
        };
        let (feedback, observation) =
            crate::npm_audit_command::observe(&args, &scope, deadline, false);
        if observation
            .as_ref()
            .is_some_and(|report| match manifest_sha256 {
                Some(expected) => report["manifest_sha256"] != expected,
                None => {
                    report["schema_version"] != "0.3.0" || report["manifest_state"] == "present"
                }
            })
        {
            result["feedback"] =
                crate::npm_audit_command::feedback("npm_manifest_input_unavailable_or_changed");
            return result;
        }
        result["feedback"] = feedback;
        if manifest_sha256.is_none() {
            result["feedback"]["next_action"] = json!(
                "历史本地记录仅定位准备范围，来源未获批准；先恢复当前清单与锁文件，再重新发现及用原工具复检，不沿用旧摘要或结果"
            );
        }
        result["observation"] = observation.unwrap_or(Value::Null);
    }
    #[cfg(not(unix))]
    {
        let _ = (root, manifest_sha256, options, deadline);
    }
    result
}

/// 串行导入并获得下一步，取消或预算耗尽时不写修复结论。
pub(crate) fn sync(root: &Path, result: &mut Value, deadline: Instant) {
    if codeguard_runtime::sigint_cancellation_requested() || Instant::now() >= deadline {
        result["backlog_status"] = json!("interrupted_not_persisted");
        return;
    }
    if !result["observation"].is_object() {
        return;
    }
    let historical = result["feedback"]["next_action"]
        .as_str()
        .is_some_and(|s| s.starts_with("历史本地记录仅定位"));
    let workbench = match crate::work_sync::save_local_report(root, &result["observation"]) {
        Ok(()) => match crate::work_sync::sync_local_workspace(root) {
            Ok(summary) => {
                json!({"status":if summary.failed_reports == 0 {"synced_partial"} else {"sync_incomplete"},"new_blockers":summary.new_blockers,"failed_reports":summary.failed_reports})
            }
            Err(reason) => json!({"status":reason}),
        },
        Err(reason) => json!({"status":reason}),
    };
    result["backlog_status"] = workbench["status"].clone();
    if result["feedback"].is_object() {
        result["feedback"]["workbench_status"] = workbench["status"].clone();
        result["feedback"]["workbench"] = workbench;
        if result["backlog_status"] == "synced_partial" {
            result["feedback"]["next_action"] = if historical {
                json!(
                    "历史本地记录仅定位准备范围，来源未获批准；运行codeguard next获取稳定任务，恢复清单后重新发现并原工具复检，不沿用旧结果或关闭任务"
                )
            } else {
                json!(
                    "运行codeguard next获取本构建根的稳定任务，先核验环境与漏洞源覆盖，再按原工具复检；零发现不自动关闭任务"
                )
            };
        }
    }
}
