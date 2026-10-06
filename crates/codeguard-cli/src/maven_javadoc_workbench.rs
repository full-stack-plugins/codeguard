//! 将Maven多文件Javadoc局部观察绑定到扫描前输入并投影为修复队列。
use crate::{
    javadoc_workbench::project_finding,
    next_command::read_local_brief_for_checker,
    work_sync::{save_local_report, sync_local_workspace},
    workspace_refresh::read_workspace_baseline,
};
use codeguard_runtime::{SourceSnapshot, read_bounded_regular_file};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
    time::{SystemTime, UNIX_EPOCH},
};

/// 保存原生观察及稳定任务；快照缺失或变化不导入旧诊断，不自动初始化或关闭。
pub(crate) fn connect(root: &Path, feedback: &Value, snapshot: Option<&SourceSnapshot>) -> Value {
    let result: Result<Value, &'static str> = (|| {
        let report = prepare(root, feedback, snapshot)?;
        save_local_report(root, &report)?;
        let summary = sync_local_workspace(root)?;
        Ok(
            json!({"status":if summary.failed_reports==0 {"synced_partial"}else{"sync_incomplete"},"new_findings":summary.new_findings,"new_blockers":summary.new_blockers,"next":read_local_brief_for_checker(root,"java.maven.javadoc").unwrap_or_else(|reason|json!({"disposition":"verification_required","reason":reason})),"task_verify_status":"local_observation_only"}),
        )
    })();
    result.unwrap_or_else(|reason|json!({"status":reason,"new_findings":0,"new_blockers":0,"next":null,"task_verify_status":"local_observation_only"}))
}

/// 绑定扫描前快照并构造可保存观察；不在此处写任务或执行工具。
pub(crate) fn prepare(
    root: &Path,
    feedback: &Value,
    snapshot: Option<&SourceSnapshot>,
) -> Result<Value, &'static str> {
    let baseline = read_workspace_baseline(root).map_err(|_| "workspace_invalid")?;
    let id = baseline
        .as_ref()
        .and_then(|b| b.workspace_id())
        .ok_or("workspace_not_initialized")?;
    let snapshot = snapshot.ok_or("maven_workbench_snapshot_unavailable")?;
    if snapshot.verify_source_unchanged().ok() != Some(true) {
        return Err("maven_workbench_inputs_changed");
    }
    let inputs: Vec<Value> = snapshot
        .files()
        .iter()
        .map(
            |(p, b)| json!({"path":p.to_string_lossy(),"sha256":format!("{:x}",Sha256::digest(b))}),
        )
        .collect();
    let native = &feedback["native_observation"];
    let (findings, blockers) = project(root, &json!(inputs), native)?;
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| "clock_unavailable")?
        .as_nanos();
    let report = json!({"schema_version":"0.1.0","report_type":"maven_javadoc_workbench_observation","workspace_binding":"bound","workspace_id":id,"run_id":format!("javadoc-maven-{}-{nanos}",std::process::id()),"checker_id":"java.maven.javadoc","authority":"local_unverified","coverage_proven":false,"delivery_decision":"not_evaluated","inputs":inputs,"native":native,"findings":findings,"blockers":blockers});
    Ok(report)
}

/// 按当前字节重算原生诊断投影；调用方同时核对保存的投影，不相信报告自填身份。
pub(crate) fn project(
    root: &Path,
    inputs: &Value,
    native: &Value,
) -> Result<(Vec<Value>, Vec<Value>), &'static str> {
    let mut bytes = BTreeMap::new();
    let mut total_bytes = 0usize;
    for input in inputs
        .as_array()
        .filter(|r| !r.is_empty() && r.len() <= 100_001)
        .ok_or("maven_inputs_invalid")?
    {
        if !exact(input, &["path", "sha256"]) {
            return Err("maven_input_invalid");
        }
        let path = input["path"]
            .as_str()
            .filter(|p| safe(p))
            .ok_or("maven_input_path_invalid")?;
        let actual = root
            .join(path)
            .canonicalize()
            .map_err(|_| "maven_input_unavailable")?;
        if !actual.starts_with(root) {
            return Err("maven_input_outside_workspace");
        }
        let data = read_bounded_regular_file(&root.join(path), 16 * 1024 * 1024)
            .map_err(|_| "maven_input_unavailable")?;
        total_bytes = total_bytes
            .checked_add(data.len())
            .filter(|n| *n <= 128 * 1024 * 1024)
            .ok_or("maven_inputs_byte_budget_exceeded")?;
        if input["sha256"] != format!("{:x}", Sha256::digest(&data)) {
            return Err("maven_input_changed");
        }
        if bytes.insert(path.to_owned(), data).is_some() {
            return Err("maven_input_duplicate");
        }
    }
    if !exact(
        native,
        &[
            "schema_version",
            "probe_mode",
            "report_type",
            "checker_id",
            "project_checker_attribution",
            "language",
            "category",
            "source_file_count",
            "observed_file_count",
            "local_probe_complete",
            "coverage_proven",
            "authority",
            "delivery_decision",
            "files",
            "maven_multifile_probes",
        ],
    ) || native["schema_version"] != "0.3.0"
        || native["report_type"] != "java_javadoc_project_probe"
        || native["probe_mode"] != "maven_multifile"
        || native["checker_id"] != "java.maven.javadoc"
        || native["language"] != "java"
        || native["category"] != "comments"
        || native["project_checker_attribution"] != "unverified"
        || native["coverage_proven"] != false
        || native["authority"] != "local_unverified"
        || native["delivery_decision"] != "not_evaluated"
    {
        return Err("maven_native_identity_invalid");
    }
    let files = native["files"]
        .as_array()
        .filter(|r| r.len() <= 100_000)
        .ok_or("maven_files_invalid")?;
    if native["source_file_count"].as_u64() != Some(files.len() as u64) {
        return Err("maven_source_count_invalid");
    }
    let probes = native["maven_multifile_probes"]
        .as_array()
        .filter(|r| r.len() <= 100_000)
        .ok_or("maven_probes_invalid")?;
    let mut seen = BTreeSet::new();
    let mut groups = BTreeMap::<String, Vec<String>>::new();
    let mut blockers = Vec::new();
    for row in files {
        if !exact(
            row,
            &[
                "path",
                "build_root",
                "configuration",
                "configuration_ref",
                "configuration_sha256",
                "reason",
                "observation",
            ],
        ) || !row["observation"].is_null()
        {
            return Err("maven_file_shape_invalid");
        }
        let path = row["path"]
            .as_str()
            .filter(|p| p.ends_with(".java") && safe(p) && bytes.contains_key(*p))
            .ok_or("maven_source_invalid")?;
        let build = row["build_root"]
            .as_str()
            .filter(|p| *p == "." || safe(p))
            .ok_or("maven_build_root_invalid")?;
        let prefix = if build == "." {
            String::new()
        } else {
            format!("{build}/")
        };
        if !path.starts_with(&prefix) || !seen.insert(path) {
            return Err("maven_source_scope_invalid");
        }
        if let Some(config) = row["configuration_ref"].as_str() {
            if config != format!("{prefix}pom.xml")
                || bytes
                    .get(config)
                    .map(|b| format!("{:x}", Sha256::digest(b)))
                    != row["configuration_sha256"].as_str().map(str::to_owned)
            {
                return Err("maven_configuration_unbound");
            }
        } else if !row["configuration_ref"].is_null() || !row["configuration_sha256"].is_null() {
            return Err("maven_configuration_invalid");
        }
        if row["reason"] == "maven_multifile_probe_selected" {
            if row["configuration"] != "configured"
                || row["configuration_ref"].is_null()
                || !path
                    .strip_prefix(&prefix)
                    .is_some_and(|p| p.starts_with("src/main/java/"))
            {
                return Err("maven_selected_scope_invalid");
            }
            groups
                .entry(build.to_owned())
                .or_default()
                .push(path.to_owned());
        } else {
            blockers.push(blocker(
                build,
                &[path.to_owned()],
                row["reason"].as_str().ok_or("maven_reason_invalid")?,
            )?);
        }
    }
    let mut used = BTreeSet::new();
    let mut findings = Vec::new();
    let mut observed = 0;
    for probe in probes {
        if !exact(probe, &["build_root", "configuration_ref", "observation"]) {
            return Err("maven_probe_shape_invalid");
        }
        let build = probe["build_root"]
            .as_str()
            .ok_or("maven_build_root_invalid")?;
        let paths = groups.get(build).ok_or("maven_probe_scope_invalid")?;
        if !used.insert(build) {
            return Err("maven_probe_duplicate");
        }
        let prefix = if build == "." {
            String::new()
        } else {
            format!("{build}/")
        };
        let config = format!("{prefix}pom.xml");
        if probe["configuration_ref"] != config {
            return Err("maven_probe_configuration_invalid");
        }
        let obs = &probe["observation"];
        let shape = crate::maven_javadoc_probe::incomplete_observation("", 0);
        if obs.as_object().map(|o| o.keys().collect::<Vec<_>>())
            != shape.as_object().map(|o| o.keys().collect::<Vec<_>>())
            || obs["schema_version"] != "0.1.0"
            || obs["report_type"] != "maven_javadoc_multifile_probe"
            || obs["checker_id"] != "java.maven.javadoc"
            || obs["project_checker_attribution"] != "unverified"
            || obs["coverage_proven"] != false
            || obs["authority"] != "local_unverified"
            || obs["delivery_decision"] != "not_evaluated"
            || obs["source_count"].as_u64() != Some(paths.len() as u64)
        {
            return Err("maven_probe_identity_invalid");
        }
        let candidates = obs["findings"]
            .as_array()
            .filter(|r| r.len() <= 100_000)
            .ok_or("maven_findings_invalid")?;
        match obs["native_status"].as_str() {
            Some("incomplete") => {
                if !candidates.is_empty() || obs["observed_source_count"] != 0 {
                    return Err("maven_incomplete_has_findings");
                }
                blockers.push(blocker(
                    build,
                    paths,
                    obs["reason"].as_str().ok_or("maven_reason_invalid")?,
                )?);
            }
            Some("findings_observed_untrusted" | "clean_log_unverified") => {
                if obs["reason"] != "direct_pom_scope_and_effective_model_unverified"
                    || !codeguard_adapters::javadoc_pom_direct_replay_eligible(
                        bytes.get(&config).ok_or("maven_configuration_unbound")?,
                    )
                    || obs["pom_mode"] != "direct_pom_replay"
                    || obs["native_plan_sha256"]
                        != format!(
                            "{:x}",
                            Sha256::digest(
                                bytes.get(&config).ok_or("maven_configuration_unbound")?
                            )
                        )
                    || obs["observed_source_count"].as_u64() != Some(paths.len() as u64)
                    || candidates.is_empty() != (obs["native_status"] == "clean_log_unverified")
                {
                    return Err("maven_probe_outcome_invalid");
                }
                for key in [
                    "maven_tool_sha256",
                    "java_runtime_sha256",
                    "dependency_closure_sha256",
                ] {
                    if !obs[key].as_str().is_some_and(sha) {
                        return Err("maven_tool_identity_invalid");
                    }
                }
                let mut ordinals = BTreeMap::<String, BTreeMap<String, u64>>::new();
                for candidate in candidates {
                    if !exact(candidate, &["path", "line", "column", "rule_id"]) {
                        return Err("maven_finding_shape_invalid");
                    }
                    let relative = candidate["path"]
                        .as_str()
                        .filter(|p| safe(p))
                        .ok_or("maven_finding_path_invalid")?;
                    let path = format!("{prefix}{relative}");
                    if !paths.contains(&path) {
                        return Err("maven_finding_scope_invalid");
                    }
                    let data = bytes.get(&path).ok_or("maven_source_unavailable")?;
                    let line = candidate["line"]
                        .as_u64()
                        .filter(|n| *n > 0)
                        .ok_or("maven_finding_line_invalid")?;
                    let text = data
                        .split(|b| *b == b'\n')
                        .nth(usize::try_from(line - 1).map_err(|_| "maven_finding_line_invalid")?)
                        .ok_or("maven_finding_line_invalid")?;
                    if !candidate["column"]
                        .as_u64()
                        .is_some_and(|c| c > 0 && c <= text.len() as u64 + 1)
                    {
                        return Err("maven_finding_column_invalid");
                    }
                    let mut fact = project_finding(
                        &path,
                        data,
                        candidate,
                        ordinals.entry(path.clone()).or_default(),
                    )
                    .ok_or("maven_finding_invalid")?;
                    let fingerprint = format!(
                        "{:x}",
                        Sha256::digest(format!(
                            "java.maven.javadoc:{}",
                            fact["finding_fingerprint"]
                                .as_str()
                                .ok_or("maven_finding_invalid")?
                        ))
                    );
                    fact["finding_fingerprint"] = json!(fingerprint);
                    fact["finding_id"] = json!(format!("CG-{}", &fingerprint[..32]));
                    findings.push(fact);
                }
                observed += paths.len();
            }
            _ => return Err("maven_status_invalid"),
        }
    }
    if groups.keys().any(|g| !used.contains(g.as_str())) {
        return Err("maven_selected_probe_missing");
    }
    if native["observed_file_count"].as_u64() != Some(observed as u64)
        || native["local_probe_complete"] != json!(observed == files.len() && !files.is_empty())
    {
        return Err("maven_observed_count_invalid");
    }
    Ok((findings, blockers))
}
fn blocker(build: &str, paths: &[String], reason: &str) -> Result<Value, &'static str> {
    if reason.is_empty()
        || !reason
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_')
    {
        return Err("maven_reason_invalid");
    }
    let fingerprint = format!(
        "{:x}",
        Sha256::digest(
            serde_json::to_vec(&json!([
                "codeguard-maven-javadoc-blocker-v1",
                build,
                reason
            ]))
            .map_err(|_| "maven_encoding_failed")?
        )
    );
    Ok(
        json!({"id":format!("CG-B-{}",&fingerprint[..32]),"fingerprint":fingerprint,"build_root":build,"scope":build,"affected_paths":paths,"reason":reason}),
    )
}
fn exact(v: &Value, keys: &[&str]) -> bool {
    v.as_object()
        .is_some_and(|o| o.len() == keys.len() && keys.iter().all(|k| o.contains_key(*k)))
}
fn safe(p: &str) -> bool {
    !p.is_empty()
        && !p.contains('\\')
        && p.split('/').all(|s| !s.is_empty() && s != "." && s != "..")
        && !p.bytes().any(|b| b < 32)
}
fn sha(s: &str) -> bool {
    s.len() == 64 && s.bytes().all(|b| b.is_ascii_hexdigit())
}
