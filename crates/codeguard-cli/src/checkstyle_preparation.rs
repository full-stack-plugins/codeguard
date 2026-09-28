//! Checkstyle 前置槽位的脱敏局部观察；准备任务不等于源码违规。
use crate::workspace_refresh::read_workspace_baseline;
use codeguard_runtime::read_bounded_regular_file;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

/// 构造环境/配置准备观察；参数为显式工作台与原失败反馈，外部目标及取消不落任务。
pub(crate) fn prepare(root: &Path, feedback: &Value) -> Result<Value, &'static str> {
    let baseline = read_workspace_baseline(root).map_err(|_| "workspace_invalid")?;
    let workspace = baseline
        .as_ref()
        .and_then(|b| b.workspace_id())
        .ok_or("workspace_not_initialized")?;
    let reason = feedback["reason"]
        .as_str()
        .ok_or("preparation_reason_invalid")?;
    if reason == "request_cancelled" {
        return Err("request_cancelled");
    }
    if !valid_reason(reason) || feedback["local_status"] != "incomplete" {
        return Err("preparation_reason_invalid");
    }
    let source = Path::new(feedback["path"].as_str().ok_or("source_path_invalid")?)
        .canonicalize()
        .map_err(|_| "source_unavailable")?;
    if source.extension().and_then(|s| s.to_str()) != Some("java") {
        return Err("source_not_java_file");
    }
    let relative = source
        .strip_prefix(root)
        .map_err(|_| "source_outside_workspace")?
        .to_str()
        .ok_or("source_path_invalid")?;
    let bytes =
        read_bounded_regular_file(&source, 16 * 1024 * 1024).map_err(|_| "source_unavailable")?;
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| "clock_unavailable")?
        .as_nanos();
    let fingerprint = fingerprint(workspace);
    Ok(
        json!({"schema_version":"0.1.0","report_type":"checkstyle_preparation_observation","workspace_binding":"bound","workspace_id":workspace,
        "run_id":format!("checkstyle-preparation-{}-{nanos}",std::process::id()),"checker_id":"java.checkstyle.preparation",
        "authority":"local_unverified","coverage_proven":false,"delivery_decision":"not_evaluated","execution":"incomplete",
        "blocker_id":format!("CG-B-{}",&fingerprint[..32]),"fingerprint":fingerprint,"reason_code":"checkstyle_prerequisites_require_review",
        "diagnostic_reason":reason,"build_root":".","scope":".","affected_paths":[relative],"source_sha256":format!("{:x}",Sha256::digest(bytes))}),
    )
}

/// 返回工作区前置槽位身份；诊断、run 与源码变化不重置任务身份。
pub(crate) fn fingerprint(workspace: &str) -> String {
    let mut hash = Sha256::new();
    for part in [
        "codeguard-checkstyle-preparation-v1",
        workspace,
        "java.checkstyle.preparation",
        ".",
    ] {
        hash.update((part.len() as u64).to_be_bytes());
        hash.update(part.as_bytes());
    }
    format!("{:x}", hash.finalize())
}

/// 恢复最新摘要绑定的准备诊断与复扫方向；不执行历史路径或可编辑 Markdown。
pub(crate) fn guidance(root: &Path, id: &str, workspace: &str) -> Result<Value, &'static str> {
    let directory = root.join("codeguard/state/observations").join(id);
    for path in [&root.join("codeguard/state/observations"), &directory] {
        if !std::fs::symlink_metadata(path).is_ok_and(|m| m.file_type().is_dir()) {
            return Err("preparation_observations_unavailable");
        }
    }
    let mut latest = None;
    for entry in std::fs::read_dir(directory).map_err(|_| "preparation_observations_unavailable")? {
        let entry = entry.map_err(|_| "preparation_observations_unavailable")?;
        let observation: Value = serde_json::from_slice(
            &read_bounded_regular_file(&entry.path(), 128 * 1024)
                .map_err(|_| "preparation_observation_unavailable")?,
        )
        .map_err(|_| "preparation_observation_invalid")?;
        let run = observation["run_id"]
            .as_str()
            .filter(|s| {
                s.starts_with("checkstyle-preparation-")
                    && s.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-')
            })
            .ok_or("preparation_observation_invalid")?;
        if observation["blocker_id"] != id
            || observation["workspace_id"] != workspace
            || observation["authority"] != "local_unverified"
        {
            return Err("preparation_observation_invalid");
        }
        let nanos = run
            .rsplit('-')
            .next()
            .and_then(|s| s.parse::<u128>().ok())
            .ok_or("preparation_observation_invalid")?;
        if latest.as_ref().is_none_or(|(old, _)| nanos > *old) {
            latest = Some((nanos, observation));
        }
    }
    let (_, observation) = latest.ok_or("preparation_observations_unavailable")?;
    let run = observation["run_id"]
        .as_str()
        .ok_or("preparation_observation_invalid")?;
    let bytes = read_bounded_regular_file(
        &root.join("codeguard/reports").join(format!("{run}.json")),
        16 * 1024 * 1024,
    )
    .map_err(|_| "preparation_report_unavailable")?;
    if observation["report_sha256"] != format!("{:x}", Sha256::digest(&bytes)) {
        return Err("preparation_report_changed");
    }
    let report: Value = serde_json::from_slice(&bytes).map_err(|_| "preparation_report_invalid")?;
    if !valid_report(&report, workspace) || report["run_id"] != run || report["blocker_id"] != id {
        return Err("preparation_report_invalid");
    }
    let path = report["affected_paths"][0]
        .as_str()
        .ok_or("preparation_scope_invalid")?;
    let source = root.join(path);
    let current = source.canonicalize().is_ok_and(|p| p.starts_with(root))
        && read_bounded_regular_file(&source, 16 * 1024 * 1024)
            .ok()
            .is_some_and(|b| report["source_sha256"] == format!("{:x}", Sha256::digest(b)));
    Ok(
        json!({"diagnostic_reason":report["diagnostic_reason"],"affected_paths":report["affected_paths"],"source_current":current,"later_native_observation":later_native_observation(root,&report)?,
        "run_id":run,"report_sha256":observation["report_sha256"],"authority":"local_unverified",
        "recheck_argv":["codeguard","lint","java",source,"--checker=checkstyle","--workspace",root,"--config","<核对后的原配置绝对路径>","--java-tool","<核对后的 Java 绝对路径>","--checkstyle-jar","<核对后的 JAR 绝对路径>","--format=json"]}),
    )
}

/// 校验局部前置观察结构及工作区归属；不证明原生执行或批准。
pub(crate) fn valid_report(report: &Value, workspace: &str) -> bool {
    let keys = [
        "schema_version",
        "report_type",
        "workspace_binding",
        "workspace_id",
        "run_id",
        "checker_id",
        "authority",
        "coverage_proven",
        "delivery_decision",
        "execution",
        "blocker_id",
        "fingerprint",
        "reason_code",
        "diagnostic_reason",
        "build_root",
        "scope",
        "affected_paths",
        "source_sha256",
    ];
    let fingerprint = fingerprint(workspace);
    report
        .as_object()
        .is_some_and(|o| o.len() == keys.len() && keys.iter().all(|k| o.contains_key(*k)))
        && report["schema_version"] == "0.1.0"
        && report["report_type"] == "checkstyle_preparation_observation"
        && report["workspace_binding"] == "bound"
        && report["run_id"].as_str().is_some_and(|s| {
            s.strip_prefix("checkstyle-preparation-")
                .and_then(|s| s.split_once('-'))
                .is_some_and(|(pid, nanos)| {
                    pid.parse::<u32>().is_ok_and(|p| p > 0)
                        && nanos.parse::<u128>().is_ok_and(|n| n > 0)
                })
        })
        && report["workspace_id"] == workspace
        && report["checker_id"] == "java.checkstyle.preparation"
        && report["authority"] == "local_unverified"
        && report["coverage_proven"] == false
        && report["delivery_decision"] == "not_evaluated"
        && report["execution"] == "incomplete"
        && report["reason_code"] == "checkstyle_prerequisites_require_review"
        && report["build_root"] == "."
        && report["scope"] == "."
        && report["fingerprint"] == fingerprint
        && report["blocker_id"] == format!("CG-B-{}", &fingerprint[..32])
        && report["diagnostic_reason"]
            .as_str()
            .is_some_and(|r| valid_reason(r) && r != "request_cancelled")
        && report["source_sha256"].as_str().is_some_and(|s| {
            s.len() == 64
                && s.bytes()
                    .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        })
        && report["affected_paths"]
            .as_array()
            .is_some_and(|a| a.len() == 1 && a[0].as_str().is_some_and(safe_path))
}
fn valid_reason(reason: &str) -> bool {
    !reason.is_empty()
        && reason.len() <= 80
        && reason.bytes().all(|b| b.is_ascii_lowercase() || b == b'_')
}
fn safe_path(path: &str) -> bool {
    !path.is_empty()
        && !path.starts_with('/')
        && !path.contains(['\\', ':'])
        && path
            .split('/')
            .all(|s| !matches!(s, "" | "." | "..") && !s.chars().any(char::is_control))
}

fn later_native_observation(root: &Path, preparation: &Value) -> Result<Value, &'static str> {
    let since = preparation["run_id"]
        .as_str()
        .and_then(|s| s.rsplit('-').next())
        .and_then(|s| s.parse::<u128>().ok())
        .ok_or("preparation_report_invalid")?;
    let mut latest: Option<(u128, Value)> = None;
    let mut count = 0;
    for entry in std::fs::read_dir(root.join("codeguard/reports"))
        .map_err(|_| "preparation_reports_unavailable")?
    {
        let entry = entry.map_err(|_| "preparation_reports_unavailable")?;
        if entry.path().extension().is_none_or(|e| e != "json") {
            continue;
        }
        count += 1;
        if count > 1000 {
            return Err("preparation_report_queue_limit_exceeded");
        }
        let bytes = read_bounded_regular_file(&entry.path(), 16 * 1024 * 1024)
            .map_err(|_| "preparation_report_unavailable")?;
        let report: Value = match serde_json::from_slice(&bytes) {
            Ok(v) => v,
            Err(_) => continue,
        };
        let Some(run) = report["run_id"].as_str().filter(|s| {
            s.starts_with("checkstyle-")
                && s.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-')
        }) else {
            continue;
        };
        let Some(sequence) = run
            .rsplit('-')
            .next()
            .and_then(|s| s.parse::<u128>().ok())
            .filter(|n| *n > since)
        else {
            continue;
        };
        let scan = if report["report_type"] == "java_checkstyle_workbench_observation" {
            &report
        } else if matches!(
            report["report_type"].as_str(),
            Some("checkstyle_task_recheck" | "checkstyle_preparation_recheck")
        ) && report["local_status"] == "observed"
        {
            &report["scan"]
        } else {
            continue;
        };
        if scan["schema_version"] != "0.1.0"
            || scan["checker_id"] != "java.checkstyle"
            || scan["authority"] != "local_unverified"
            || scan["coverage_proven"] != false
            || scan["delivery_decision"] != "not_evaluated"
            || scan["workspace_id"] != preparation["workspace_id"]
            || scan["source_path"] != preparation["affected_paths"][0]
            || scan["run_id"] != run
        {
            continue;
        }
        let marker = root
            .join("codeguard/state/consumed")
            .join(format!("{run}.json"));
        let marker: Value = match read_bounded_regular_file(&marker, 4096)
            .ok()
            .and_then(|b| serde_json::from_slice(&b).ok())
        {
            Some(v) => v,
            None => continue,
        };
        let digest = format!("{:x}", Sha256::digest(&bytes));
        if marker["workspace_id"] != preparation["workspace_id"]
            || marker["run_id"] != run
            || marker["report_sha256"] != digest
        {
            continue;
        }
        let stable = [
            ("source", 16 * 1024 * 1024),
            ("config", 1024 * 1024),
            ("java", 128 * 1024 * 1024),
            ("jar", 128 * 1024 * 1024),
        ]
        .iter()
        .all(|(key, limit)| {
            let input = &scan["inputs"][*key];
            input["path"].as_str().is_some_and(|p| {
                read_bounded_regular_file(Path::new(p), *limit)
                    .ok()
                    .is_some_and(|b| input["sha256"] == format!("{:x}", Sha256::digest(b)))
            })
        });
        if stable && latest.as_ref().is_none_or(|(old, _)| sequence > *old) {
            latest = Some((
                sequence,
                json!({"run_id":run,"report_sha256":digest,"status":"same_scope_native_observed_unverified","authority":"local_unverified"}),
            ));
        }
    }
    Ok(latest.map_or(Value::Null, |(_, value)| value))
}
