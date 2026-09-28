//! 将显式工作台中的 Checkstyle 局部观察连接到既有修复队列。
use crate::next_command::read_local_brief_for_checker;
use crate::work_sync::{save_local_report, sync_local_workspace};
use crate::workspace_refresh::read_workspace_baseline;
use codeguard_runtime::read_bounded_regular_file;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

/// 保存并同步原工具局部观察；参数为显式工作台、反馈及冻结输入摘要。
/// 返回本地队列状态，不签发交付许可，也不关闭旧任务。
pub(crate) fn connect(root: &Path, feedback: &Value, inputs: &Value) -> Value {
    let prepared = if feedback["local_status"] == "incomplete" {
        crate::checkstyle_preparation::prepare(root, feedback)
    } else {
        prepare(root, feedback, inputs)
    };
    match prepared {
        Ok(report) => {
            if let Err(reason) = save_local_report(root, &report) {
                return json!({"status":reason,"new_findings":0,"new_blockers":0,"next":null});
            }
            match sync_local_workspace(root) {
                Ok(summary) => {
                    json!({"status":if summary.failed_reports == 0 {"synced_partial"} else {"sync_incomplete"},
                    "new_findings":summary.new_findings,"new_blockers":summary.new_blockers,"next":match read_local_brief_for_checker(root,report["checker_id"].as_str().unwrap_or("java.checkstyle")) {
                        Ok(next)=>next,
                        Err(reason)=>json!({"disposition":"verification_required","reason":reason,"authority":"local_unverified","delivery_decision":"not_evaluated","next_actions":["inspect_workbench_records_and_repeat_native_check"]})
                    }})
                }
                Err(reason) => {
                    json!({"status":reason,"new_findings":0,"new_blockers":0,"next":null})
                }
            }
        }
        Err(reason) => json!({"status":reason,"new_findings":0,"new_blockers":0,"next":null}),
    }
}

/// 从已观察原输入构造本地队列报告；错误不生成源码任务。
pub(crate) fn prepare(
    root: &Path,
    feedback: &Value,
    inputs: &Value,
) -> Result<Value, &'static str> {
    let baseline = read_workspace_baseline(root).map_err(|_| "workspace_invalid")?;
    let workspace_id = baseline
        .as_ref()
        .and_then(|b| b.workspace_id())
        .ok_or("workspace_not_initialized")?;
    if feedback["local_status"] != "observed" {
        return Err("native_observation_incomplete");
    }
    let source = Path::new(feedback["path"].as_str().ok_or("source_path_invalid")?);
    let source = source.canonicalize().map_err(|_| "source_unavailable")?;
    let relative = source
        .strip_prefix(root)
        .map_err(|_| "source_outside_workspace")?
        .to_str()
        .ok_or("source_path_invalid")?;
    let bytes =
        read_bounded_regular_file(&source, 16 * 1024 * 1024).map_err(|_| "source_unavailable")?;
    if inputs["source"]["sha256"] != format!("{:x}", Sha256::digest(&bytes)) {
        return Err("original_input_changed");
    }
    let mut occurrences = BTreeMap::new();
    let findings = feedback["findings"]
        .as_array()
        .ok_or("finding_projection_incomplete")?
        .iter()
        .map(|f| project_finding(relative, &bytes, f, &mut occurrences))
        .collect::<Option<Vec<_>>>()
        .ok_or("finding_projection_incomplete")?;
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| "clock_unavailable")?
        .as_nanos();
    Ok(
        json!({"schema_version":"0.1.0","report_type":"java_checkstyle_workbench_observation",
        "workspace_binding":"bound","workspace_id":workspace_id,"run_id":format!("checkstyle-{}-{nanos}",std::process::id()),
        "authority":"local_unverified","coverage_proven":false,"delivery_decision":"not_evaluated",
        "checker_id":"java.checkstyle","source_path":relative,"inputs":inputs,"findings":findings}),
    )
}

/// 基于原生规则与源码锚点生成稳定身份；行号只用于证据定位。
/// 参数包含相对路径、冻结源码、原生反馈及同锚点出现序号；无法定位返回空。
pub(crate) fn project_finding(
    relative: &str,
    bytes: &[u8],
    native: &Value,
    occurrences: &mut BTreeMap<String, u64>,
) -> Option<Value> {
    let line = native["line"].as_u64().filter(|n| *n > 0)?;
    let anchor = bytes
        .split(|b| *b == b'\n')
        .nth(usize::try_from(line - 1).ok()?)?
        .trim_ascii();
    if anchor.is_empty() {
        return None;
    }
    let rule = native["rule_id"].as_str()?;
    let class = native["checker_class"].as_str()?;
    let mut hash = Sha256::new();
    for part in [
        b"codeguard-checkstyle-finding-v1".as_slice(),
        relative.as_bytes(),
        rule.as_bytes(),
        class.as_bytes(),
        anchor,
    ] {
        hash.update((part.len() as u64).to_be_bytes());
        hash.update(part);
    }
    let base = format!("{:x}", hash.finalize());
    let ordinal = occurrences.entry(base.clone()).or_default();
    let mut hash = Sha256::new();
    hash.update(base.as_bytes());
    hash.update(ordinal.to_be_bytes());
    *ordinal += 1;
    let fingerprint = format!("{:x}", hash.finalize());
    Some(
        json!({"finding_id":format!("CG-{}",&fingerprint[..32]),"finding_fingerprint":fingerprint,
        "source_sha256":format!("{:x}",Sha256::digest(bytes)),"path":relative,"rule_id":rule,"line":line,"native":native}),
    )
}

/// 从摘要绑定的最新局部观察恢复修复依据；不读取可编辑任务中的指令。
/// 参数为工作台、稳定任务身份及工作区身份；配置变化仅要求复扫。
pub(crate) fn task_guidance(
    root: &Path,
    id: &str,
    workspace_id: &str,
) -> Result<Value, &'static str> {
    let directory = root.join("codeguard/state/observations").join(id);
    for path in [&root.join("codeguard/state/observations"), &directory] {
        if !std::fs::symlink_metadata(path).is_ok_and(|m| m.file_type().is_dir()) {
            return Err("checkstyle_observations_unavailable");
        }
    }
    let mut latest = None;
    for entry in std::fs::read_dir(directory).map_err(|_| "checkstyle_observations_unavailable")? {
        let entry = entry.map_err(|_| "checkstyle_observations_unavailable")?;
        let bytes = read_bounded_regular_file(&entry.path(), 128 * 1024)
            .map_err(|_| "checkstyle_observation_unavailable")?;
        let observation: Value =
            serde_json::from_slice(&bytes).map_err(|_| "checkstyle_observation_invalid")?;
        let run = observation["run_id"]
            .as_str()
            .ok_or("checkstyle_observation_invalid")?;
        if !run.starts_with("checkstyle-")
            || !run.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-')
            || observation["workspace_id"] != workspace_id
            || observation["finding_id"] != id
            || observation["authority"] != "local_unverified"
        {
            return Err("checkstyle_observation_invalid");
        }
        let time = run
            .rsplit('-')
            .next()
            .and_then(|s| s.parse::<u128>().ok())
            .ok_or("checkstyle_observation_invalid")?;
        if latest.as_ref().is_none_or(|(old, _)| time > *old) {
            latest = Some((time, observation));
        }
    }
    let (_, observation) = latest.ok_or("checkstyle_observations_unavailable")?;
    let run = observation["run_id"]
        .as_str()
        .ok_or("checkstyle_observation_invalid")?;
    let bytes = read_bounded_regular_file(
        &root.join("codeguard/reports").join(format!("{run}.json")),
        16 * 1024 * 1024,
    )
    .map_err(|_| "checkstyle_report_unavailable")?;
    if observation["report_sha256"] != format!("{:x}", Sha256::digest(&bytes)) {
        return Err("checkstyle_report_changed");
    }
    let report: Value = serde_json::from_slice(&bytes).map_err(|_| "checkstyle_report_invalid")?;
    let report = if matches!(
        report["report_type"].as_str(),
        Some("checkstyle_task_recheck" | "checkstyle_preparation_recheck")
    ) {
        report["scan"].clone()
    } else {
        report
    };
    if report["report_type"] != "java_checkstyle_workbench_observation"
        || report["run_id"] != run
        || report["workspace_id"] != workspace_id
    {
        return Err("checkstyle_report_invalid");
    }
    let finding = report["findings"]
        .as_array()
        .and_then(|a| a.iter().find(|f| f["finding_id"] == id))
        .ok_or("checkstyle_finding_unavailable")?;
    let fact: Value = serde_json::from_slice(
        &read_bounded_regular_file(
            &root
                .join("codeguard/findings")
                .join(id)
                .join("finding.json"),
            128 * 1024,
        )
        .map_err(|_| "checkstyle_first_fact_unavailable")?,
    )
    .map_err(|_| "checkstyle_first_fact_invalid")?;
    let first_run = fact["first_run_id"]
        .as_str()
        .filter(|s| !s.is_empty() && s.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-'))
        .ok_or("checkstyle_first_fact_invalid")?;
    let first_bytes = read_bounded_regular_file(
        &root
            .join("codeguard/reports")
            .join(format!("{first_run}.json")),
        16 * 1024 * 1024,
    )
    .map_err(|_| "checkstyle_first_report_unavailable")?;
    if fact["first_report_sha256"] != format!("{:x}", Sha256::digest(&first_bytes)) {
        return Err("checkstyle_first_report_changed");
    }
    let first: Value =
        serde_json::from_slice(&first_bytes).map_err(|_| "checkstyle_first_report_invalid")?;
    let first = if matches!(
        first["report_type"].as_str(),
        Some("checkstyle_task_recheck" | "checkstyle_preparation_recheck")
    ) {
        first["scan"].clone()
    } else {
        first
    };
    let original_tool_inputs = json!({"java":first["inputs"]["java"],"jar":first["inputs"]["jar"]});
    let native = &finding["native"];
    let inputs = &report["inputs"];
    let config = Path::new(
        inputs["config"]["path"]
            .as_str()
            .ok_or("checkstyle_configuration_unavailable")?,
    );
    let config_current = read_bounded_regular_file(config, 1024 * 1024)
        .ok()
        .is_some_and(|bytes| inputs["config"]["sha256"] == format!("{:x}", Sha256::digest(bytes)));
    Ok(
        json!({"rule_summary":native["rule_summary"],"checker_class":native["checker_class"],"rule_reference":native["rule_reference"],"repair_steps":native["repair_steps"],
        "source_sha256":finding["source_sha256"],"configuration_current":config_current,"observed_inputs":inputs,"original_tool_inputs":original_tool_inputs,
        "original_configuration_input":first["inputs"]["config"],
        "recheck_argv":["codeguard","lint","java",inputs["source"]["path"],"--checker=checkstyle","--workspace",root,
            "--config",inputs["config"]["path"],"--java-tool",inputs["java"]["path"],"--checkstyle-jar",inputs["jar"]["path"],"--format=json"]}),
    )
}
