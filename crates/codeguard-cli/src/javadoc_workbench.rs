//! 将 Javadoc 局部观察接入持久修复队列，不授予交付与自动关闭资格。
use crate::next_command::read_local_brief_for_checker;
use crate::work_sync::{save_local_report, sync_local_workspace};
use crate::workspace_refresh::read_workspace_baseline;
use codeguard_runtime::read_bounded_regular_file;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

/// 对已初始化项目保存当前局部观察；错误保持可见，不创建工作区或关闭问题。
pub(crate) fn connect(root: &Path, feedback: &Value) -> Value {
    let result = prepare(root, feedback).and_then(|report| {
        save_local_report(root, &report)?;
        let summary = sync_local_workspace(root)?;
        Ok(json!({"status":if summary.failed_reports == 0 {"synced_partial"} else {"sync_incomplete"},"new_findings":summary.new_findings,"new_blockers":summary.new_blockers,"next":read_local_brief_for_checker(root,"java.jdk.javadoc").unwrap_or_else(|reason| json!({"disposition":"verification_required","reason":reason})),"task_verify_status":"local_observation_only"}))
    });
    result.unwrap_or_else(|reason| json!({"status":reason,"new_findings":0,"new_blockers":0,"next":null,"task_verify_status":"local_observation_only"}))
}

pub(crate) fn prepare(root: &Path, feedback: &Value) -> Result<Value, &'static str> {
    let baseline = read_workspace_baseline(root).map_err(|_| "workspace_invalid")?;
    let id = baseline
        .as_ref()
        .and_then(|b| b.workspace_id())
        .ok_or("workspace_not_initialized")?;
    let native = &feedback["native_observation"];
    let explicit = native["report_type"] == "java_javadoc_local_feedback";
    let rows = if explicit {
        let source = Path::new(native["path"].as_str().ok_or("javadoc_path_invalid")?)
            .canonicalize()
            .map_err(|_| "javadoc_source_unavailable")?;
        let relative = source
            .strip_prefix(root)
            .map_err(|_| "source_outside_workspace")?
            .to_str()
            .ok_or("javadoc_path_invalid")?;
        if !relative.ends_with(".java") {
            return Err("javadoc_path_invalid");
        }
        vec![
            json!({"path":relative,"reason":native["reason"],"configuration_ref":null,"configuration_sha256":null,"observation":native}),
        ]
    } else {
        if native["report_type"] != "java_javadoc_project_probe"
            || native["probe_mode"] != "jdk_single_file"
        {
            return Err("javadoc_workbench_scope_not_integrated");
        }
        native["files"]
            .as_array()
            .filter(|r| r.len() <= 100_000)
            .ok_or("javadoc_observations_invalid")?
            .clone()
    };
    let mut sources = Vec::new();
    for row in rows {
        let relative = row["path"].as_str().ok_or("javadoc_path_invalid")?;
        let bytes = read_bounded_regular_file(&root.join(relative), 16 * 1024 * 1024)
            .map_err(|_| "javadoc_source_unavailable")?;
        let reason = row["reason"].as_str().ok_or("javadoc_reason_invalid")?;
        let mut findings = Vec::new();
        let observation = &row["observation"];
        if !observation.is_null() {
            if observation["schema_version"] != "0.2.0" {
                return Err("javadoc_native_protocol_invalid");
            }
            if !observation["source_sha256"].is_null()
                && observation["source_sha256"] != format!("{:x}", Sha256::digest(&bytes))
            {
                return Err("javadoc_source_changed");
            }
            if matches!(
                observation["local_status"].as_str(),
                Some("findings_observed_untrusted" | "clean_scope_unproven")
            ) {
                let mut occurrences = BTreeMap::new();
                for native in observation["findings"]
                    .as_array()
                    .ok_or("javadoc_findings_invalid")?
                {
                    findings.push(
                        project_finding(relative, &bytes, native, &mut occurrences)
                            .ok_or("javadoc_finding_invalid")?,
                    );
                }
            }
        }
        sources.push(json!({"path":relative,"source_sha256":format!("{:x}",Sha256::digest(&bytes)),"reason":reason,"configuration_ref":row["configuration_ref"],"configuration_sha256":row["configuration_sha256"],"native":observation,"findings":findings}));
    }
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| "clock_unavailable")?
        .as_nanos();
    Ok(
        json!({"schema_version":"0.3.0","observation_scope":if explicit {"explicit_file_probe"} else {"configured_project_probe"},"report_type":"javadoc_workbench_observation","workspace_binding":"bound","workspace_id":id,"run_id":format!("javadoc-{}-{nanos}",std::process::id()),"checker_id":"java.jdk.javadoc","authority":"local_unverified","coverage_proven":false,"delivery_decision":"not_evaluated","sources":sources}),
    )
}

/// 从固定原生规则及当前源码锚点构造问题身份；行号仅用于定位。
pub(crate) fn project_finding(
    relative: &str,
    bytes: &[u8],
    native: &Value,
    occurrences: &mut BTreeMap<String, u64>,
) -> Option<Value> {
    let rule = native["rule_id"].as_str()?;
    if !matches!(
        rule,
        "JavadocMissingComment"
            | "JavadocDefaultConstructorMissingComment"
            | "JavadocMissingParam"
            | "JavadocMissingReturn"
            | "JavadocMissingThrows"
            | "JavadocEmptyComment"
            | "JavadocMissingMainDescription"
            | "JavadocEmptyParamDescription"
            | "JavadocEmptyReturnDescription"
            | "JavadocEmptyThrowsDescription"
    ) {
        return None;
    }
    let line = native["line"].as_u64().filter(|line| *line > 0)?;
    let anchor = bytes
        .split(|b| *b == b'\n')
        .nth(usize::try_from(line - 1).ok()?)?
        .trim_ascii();
    if anchor.is_empty() {
        return None;
    }
    let mut hash = Sha256::new();
    for part in [
        b"codeguard-javadoc-finding-v1".as_slice(),
        relative.as_bytes(),
        rule.as_bytes(),
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
        json!({"finding_id":format!("CG-{}",&fingerprint[..32]),"finding_fingerprint":fingerprint,"path":relative,"source_sha256":format!("{:x}",Sha256::digest(bytes)),"rule_id":rule,"line":line}),
    )
}
