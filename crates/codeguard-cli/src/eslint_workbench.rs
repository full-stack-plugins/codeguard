//! ESLint 局部原生观察到既有修复队列的连接；没有审批或关闭权威。
use crate::{
    eslint_lint_arguments::EslintLintArguments,
    eslint_probe_request::EslintProbeRequest,
    eslint_probe_result::EslintProbeResult,
    work_sync::{save_local_report, sync_local_workspace},
    workspace_refresh::read_workspace_baseline,
};
use codeguard_adapters::project_eslint_findings;
use codeguard_runtime::read_bounded_regular_file;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::path::Path;

pub(crate) fn connect(
    root: &Path,
    request: &EslintProbeRequest,
    result: &EslintProbeResult,
) -> Value {
    match prepare(root, request, result) {
        Ok(report) => persist(root, report, "node.eslint"),
        Err(reason) => json!({"status":reason}),
    }
}
pub(crate) fn connect_preparation(root: &Path, args: &EslintLintArguments, reason: &str) -> Value {
    match crate::eslint_preparation::prepare(root, args, reason) {
        Ok(report) => {
            let id = report["blocker_id"].as_str().unwrap_or("").to_owned();
            let mut result = persist(root, report, "node.eslint.preparation");
            if matches!(
                reason,
                "eslint_syntax_confirmation_needed" | "eslint_syntax_precheck_unavailable"
            ) && result["status"] == "synced_partial"
            {
                if let Ok(brief) = crate::next_command::read_task_brief(root, &id) {
                    result["task_id"] = json!(id);
                    result["next"] = json!({
                        "schema_version":"0.1.0","report_type":"repair_brief_preview",
                        "operation":"next","command_status":"complete","exit_code":0,
                        "disposition":brief["disposition"],"reason":"current_source_confirmation_task",
                        "repair_brief":brief,"next_actions":[],
                        "authority":"local_unverified","delivery_decision":"not_evaluated"
                    });
                }
            }
            result
        }
        Err(reason) => json!({"status":reason}),
    }
}
fn persist(root: &Path, report: Value, checker: &str) -> Value {
    if let Err(reason) = save_local_report(root, &report) {
        return json!({"status":reason});
    }
    match sync_local_workspace(root) {
        Ok(summary) => {
            let (next, next_error) =
                match crate::next_command::read_local_brief_for_checker(root, checker) {
                    Ok(next) => (Some(next), None),
                    Err(reason) => (None, Some(reason)),
                };
            json!({"status":if summary.failed_reports==0 {"synced_partial"} else {"sync_incomplete"},"new_findings":summary.new_findings,"new_blockers":summary.new_blockers,"failed_reports":summary.failed_reports,"next":next,"next_error":next_error})
        }
        Err(reason) => json!({"status":reason}),
    }
}
pub(crate) fn prepare(
    root: &Path,
    request: &EslintProbeRequest,
    result: &EslintProbeResult,
) -> Result<Value, &'static str> {
    let baseline = read_workspace_baseline(root).map_err(|_| "workspace_invalid")?;
    let id = baseline
        .as_ref()
        .and_then(|value| value.workspace_id())
        .ok_or("workspace_not_initialized")?;
    let source = request
        .command
        .sources
        .first()
        .filter(|_| request.command.sources.len() == 1)
        .ok_or("eslint_scope_invalid")?;
    let relative = source
        .strip_prefix(root)
        .ok()
        .and_then(Path::to_str)
        .filter(|path| !path.is_empty())
        .ok_or("source_outside_workspace")?;
    let mut inputs = json!({});
    let mut source_bytes = Vec::new();
    for (name, path, limit) in [
        ("source", source, 16 * 1024 * 1024),
        ("config", &request.command.config, 1024 * 1024),
        ("node", &request.command.node, 128 * 1024 * 1024),
        ("eslint", &request.command.entry, 16 * 1024 * 1024),
    ] {
        let bytes =
            read_bounded_regular_file(path, limit).map_err(|_| "eslint_input_unavailable")?;
        let sha: [u8; 32] = Sha256::digest(&bytes).into();
        if path.canonicalize().ok().as_ref() != Some(path)
            || request.expected_sha256.get(path) != Some(&sha)
        {
            return Err("eslint_input_changed");
        }
        inputs[name] = json!({"path":path,"sha256":format!("{:x}",Sha256::digest(&bytes))});
        if name == "source" {
            source_bytes = bytes;
        }
    }
    let findings = if result.local_coherent {
        project_eslint_findings(
            relative,
            source.to_str().ok_or("source_path_invalid")?,
            &source_bytes,
            &result
                .parsed
                .as_ref()
                .ok_or("eslint_report_missing")?
                .findings,
        )
        .ok_or("eslint_finding_projection_invalid")?
    } else {
        Vec::new()
    };
    Ok(
        json!({"schema_version":"0.1.0","report_type":"eslint_workbench_observation","workspace_binding":"bound","workspace_id":id,"run_id":request.run_id,"authority":"local_unverified","coverage_proven":false,"delivery_decision":"not_evaluated","checker_id":"node.eslint","source_path":relative,"inputs":inputs,"cwd":request.cwd,"native_version":request.expected_version,"local_coherent":result.local_coherent,"reason":result.reason,"findings":findings}),
    )
}

pub(crate) fn guidance(root: &Path, fact: &Value) -> Value {
    let fallback = json!({"disposition":"verification_required","step":"ESLint 当前报告或输入无法核对；重新运行原工具，不按旧任务盲目修改源码"});
    let Some(scope) = fact["path"].as_str() else {
        return fallback;
    };
    let Ok(entries) = std::fs::read_dir(root.join(".codeguard/reports")) else {
        return fallback;
    };
    let mut latest = None;
    let mut count = 0;
    for entry in entries {
        count += 1;
        if count > 1000 {
            return fallback;
        }
        let Ok(entry) = entry else {
            return fallback;
        };
        let Ok(bytes) = read_bounded_regular_file(&entry.path(), 16 * 1024 * 1024) else {
            return fallback;
        };
        let Ok(mut report) = serde_json::from_slice::<Value>(&bytes) else {
            return fallback;
        };
        if report["report_type"] == "eslint_task_recheck" {
            if crate::eslint_task_recheck::valid_shape(&report) && report["scan"].is_object() {
                report = report["scan"].clone();
            } else {
                report["source_path"] = report["target"]["path"].clone();
            }
        }
        if !matches!(
            report["report_type"].as_str(),
            Some("eslint_workbench_observation" | "eslint_task_recheck")
        ) || report["workspace_id"] != fact["workspace_id"]
            || report["source_path"] != scope
        {
            continue;
        }
        let Some(run) = report["run_id"].as_str() else {
            return fallback;
        };
        let Some(sequence) = run
            .rsplit('-')
            .next()
            .and_then(|value| value.parse::<u128>().ok())
        else {
            return fallback;
        };
        if latest.as_ref().is_none_or(|(old, _, _, _)| sequence > *old) {
            latest = Some((sequence, entry.path(), report, bytes));
        }
    }
    let Some((_, path, report, bytes)) = latest else {
        return fallback;
    };
    let sha = format!("{:x}", Sha256::digest(&bytes));
    let effective = serde_json::from_slice::<Value>(&bytes)
        .ok()
        .filter(|outer| {
            outer["report_type"] == "eslint_task_recheck"
                && crate::eslint_task_recheck::valid_shape(outer)
                && outer["effective_rule"]["rule_id"] == fact["native_rule_id"]
        })
        .map_or(Value::Null, |outer| outer["effective_rule"].clone());
    let run = report["run_id"].as_str().unwrap();
    let receipt = root
        .join(".codeguard/state/consumed")
        .join(format!("{run}.json"));
    let valid_receipt = read_bounded_regular_file(&receipt, 4096)
        .ok()
        .and_then(|bytes| serde_json::from_slice::<Value>(&bytes).ok())
        .is_some_and(|value| {
            value["workspace_id"] == fact["workspace_id"]
                && value["run_id"] == run
                && value["report_sha256"] == sha
        });
    if !valid_receipt
        || !crate::work_sync::validate_eslint_observation(
            root,
            fact["workspace_id"].as_str().unwrap_or(""),
            &path,
            &report,
            sha,
        )
    {
        return fallback;
    }
    let first = root.join(".codeguard/reports").join(format!(
        "{}.json",
        fact["first_run_id"].as_str().unwrap_or("")
    ));
    let original = read_bounded_regular_file(&first, 16 * 1024 * 1024)
        .ok()
        .filter(|bytes| fact["first_report_sha256"] == format!("{:x}", Sha256::digest(bytes)))
        .and_then(|bytes| serde_json::from_slice::<Value>(&bytes).ok())
        .map(|report| {
            if report["report_type"] == "eslint_task_recheck"
                && crate::eslint_task_recheck::valid_shape(&report)
            {
                report["scan"].clone()
            } else {
                report
            }
        });
    let unchanged = original.is_some_and(|old| {
        ["config", "node", "eslint"]
            .iter()
            .all(|key| old["inputs"][*key] == report["inputs"][*key])
            && old["cwd"] == report["cwd"]
            && old["native_version"] == report["native_version"]
    });
    let present = report["findings"].as_array().is_some_and(|findings| {
        findings
            .iter()
            .any(|finding| finding["finding_id"] == fact["id"])
    });
    let (disposition, step) = if !unchanged {
        (
            "verification_required",
            "ESLint 工具或原配置与首次观察不同；核查原规则覆盖后复检，不沿用旧修复或零诊断结论",
        )
    } else if report["reason"] == "eslint_suppression_requires_review" {
        (
            "verification_required",
            "原生注释抑制影响了诊断；核对抑制与批准依据，不把任务勾选或零诊断当修复",
        )
    } else if report["local_coherent"] != true {
        (
            "verification_required",
            "ESLint 原工具检查未完成；先核对当前报告原因和检查环境，不修改无关源码",
        )
    } else if present {
        (
            "actionable",
            "按当前 ESLint 原生规则核对目标语义并修复；不关闭规则、不添加抑制，随后用同一原配置复检",
        )
    } else if effective["status"] == "observed"
        && (effective["severity"].is_null() || effective["severity"] == 0)
    {
        (
            "verification_required",
            "ESLint原生有效配置未启用原规则；先核对配置与批准依据，不把零诊断当修复、不修改无关源码",
        )
    } else if effective.is_object() && effective["status"] != "observed" {
        (
            "verification_required",
            "ESLint有效配置查询未完成；恢复查询并核对原规则覆盖，不把局部零诊断当修复",
        )
    } else {
        (
            "verification_required",
            "本轮局部原生结果未发现旧问题；核对原规则覆盖及正式复检，不重复旧源码修复，任务保持开放",
        )
    };
    json!({"disposition":disposition,"step":step,"report_run_id":run,"effective_rule":effective,"recheck_argv":["codeguard","lint","typescript",scope,"--workspace",root,"--node-tool",report["inputs"]["node"]["path"],"--eslint-entry",report["inputs"]["eslint"]["path"],"--eslint-version",report["native_version"],"--config",report["inputs"]["config"]["path"],"--cwd",report["cwd"]]})
}
