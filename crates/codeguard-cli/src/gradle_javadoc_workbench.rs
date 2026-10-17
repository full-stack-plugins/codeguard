//! Gradle文档观察的输入绑定与稳定问题投影；不执行报告内命令，不授予可信关闭。
use codeguard_runtime::SourceSnapshot;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::{Path, PathBuf},
};

/// 核验当前选定输入，输出独立Gradle问题和准备观察；参数为项目根、路径/摘要数组和局部原生报告。
/// 返回稳定投影；空发现仍保留规则/范围未验收观察，此函数不写工作区或关闭任务。
pub fn project(
    root: &Path,
    inputs: &Value,
    native: &Value,
) -> Result<(Vec<Value>, Vec<Value>), &'static str> {
    let rows = inputs
        .as_array()
        .filter(|r| !r.is_empty() && r.len() <= 128)
        .ok_or("gradle_inputs_invalid")?;
    let mut paths = BTreeSet::new();
    let mut source_digests = BTreeMap::new();
    for row in rows {
        if !exact(row, &["path", "sha256"]) || !row["sha256"].as_str().is_some_and(sha) {
            return Err("gradle_input_invalid");
        }
        let path = row["path"]
            .as_str()
            .filter(|p| safe(p))
            .ok_or("gradle_input_path_invalid")?;
        source_digests.insert(path, row["sha256"].as_str().ok_or("gradle_input_invalid")?);
        if !paths.insert(PathBuf::from(path)) {
            return Err("gradle_input_duplicate");
        }
    }
    if !["settings.gradle", "settings.gradle.kts"]
        .iter()
        .any(|p| paths.contains(Path::new(p)))
        || !["build.gradle", "build.gradle.kts"]
            .iter()
            .any(|p| paths.contains(Path::new(p)))
    {
        return Err("gradle_root_inputs_missing");
    }
    let snapshot = SourceSnapshot::capture(root, paths, 128, 1024 * 1024, 16 * 1024 * 1024)
        .map_err(|_| "gradle_inputs_unavailable")?;
    let actual: Vec<Value> = snapshot
        .files()
        .iter()
        .map(|(p, b)| json!({"path":p,"sha256":digest(b)}))
        .collect();
    if inputs != &json!(actual) {
        return Err("gradle_input_changed_or_noncanonical");
    }
    valid_native(native)?;
    let input_digest =
        digest(&serde_json::to_vec(inputs).map_err(|_| "gradle_inputs_encoding_invalid")?);
    if !native["source_snapshot_sha256"].is_null()
        && native["source_snapshot_sha256"] != input_digest
    {
        return Err("gradle_native_input_binding_invalid");
    }
    let java_paths = snapshot
        .files()
        .keys()
        .filter(|p| p.extension().is_some_and(|e| e == "java"))
        .map(|p| p.to_string_lossy().into_owned())
        .collect::<Vec<_>>();
    if java_paths.is_empty() {
        return Err("gradle_java_inputs_missing");
    }
    let observed = native["native_status"] != "incomplete";
    if observed
        && (native["source_snapshot_sha256"].is_null()
            || native["selected_java_file_count"].as_u64() != Some(java_paths.len() as u64))
    {
        return Err("gradle_native_source_scope_invalid");
    }
    // 只索引诊断引用的行；每份源码扫描一次，避免大量定位重复遍历大文件。
    let mut wanted_lines = BTreeMap::<&str, BTreeSet<u64>>::new();
    for row in native["findings"]
        .as_array()
        .ok_or("gradle_findings_invalid")?
    {
        wanted_lines
            .entry(row["path"].as_str().ok_or("gradle_path_invalid")?)
            .or_default()
            .insert(row["line"].as_u64().ok_or("gradle_line_invalid")?);
    }
    let mut source_lines = BTreeMap::new();
    for (path, wanted) in wanted_lines {
        let bytes = snapshot
            .files()
            .get(Path::new(path))
            .ok_or("gradle_finding_outside_inputs")?;
        for (offset, raw) in bytes.split(|b| *b == b'\n').enumerate() {
            let line = offset as u64 + 1;
            if wanted.contains(&line) {
                source_lines.insert((path, line), raw);
            }
        }
    }
    let mut findings = Vec::new();
    let mut locations = BTreeSet::new();
    let mut occurrences = BTreeMap::<String, u64>::new();
    let mut anchors = BTreeMap::<(&str, &str, u64), String>::new();
    for row in native["findings"]
        .as_array()
        .ok_or("gradle_findings_invalid")?
    {
        let path = row["path"]
            .as_str()
            .filter(|p| java_paths.iter().any(|s| s == p))
            .ok_or("gradle_finding_outside_inputs")?;
        let rule = row["rule_id"].as_str().ok_or("gradle_rule_invalid")?;
        let line = row["line"]
            .as_u64()
            .filter(|n| *n > 0)
            .ok_or("gradle_line_invalid")?;
        let column = row["column"]
            .as_u64()
            .filter(|n| *n > 0)
            .ok_or("gradle_column_invalid")?;
        let raw_line = source_lines
            .get(&(path, line))
            .ok_or("gradle_line_invalid")?;
        if column > raw_line.len() as u64 + 1 || raw_line.trim_ascii().is_empty() {
            return Err("gradle_location_invalid");
        }
        // 多个官方任务可重复检查相同源集；保留原报告证据，只归并相同定位。
        if !locations.insert((path.to_owned(), rule.to_owned(), line, column)) {
            continue;
        }
        let base = anchors
            .entry((path, rule, line))
            .or_insert_with(|| {
                identity(&[
                    b"codeguard-gradle-javadoc-finding-v1",
                    path.as_bytes(),
                    rule.as_bytes(),
                    raw_line.trim_ascii(),
                ])
            })
            .clone();
        let occurrence = occurrences.entry(base.clone()).or_default();
        let fingerprint = identity(&[base.as_bytes(), &occurrence.to_be_bytes()]);
        *occurrence += 1;
        findings.push(json!({"checker_id":"java.gradle.javadoc","finding_id":format!("CG-{}",&fingerprint[..32]),"finding_fingerprint":fingerprint,"path":path,"source_sha256":source_digests[path],"rule_id":rule,"line":line,"column":column}));
    }
    // 准备问题按检查器和所选路径集稳定归并，取消/工具故障变化只更新诊断。
    let blocker_fingerprint = identity(&[
        b"codeguard-gradle-javadoc-blocker-v1",
        &serde_json::to_vec(&java_paths).map_err(|_| "gradle_paths_encoding_invalid")?,
    ]);
    let blockers = vec![
        json!({"checker_id":"java.gradle.javadoc","id":format!("CG-B-{}",&blocker_fingerprint[..32]),"fingerprint":blocker_fingerprint,"reason":if observed {"gradle_documentation_rules_and_coverage_unverified"}else{"gradle_javadoc_native_incomplete"},"diagnostic_reason":native["reason"],"build_root":".","scope":".","affected_paths":java_paths}),
    ];
    if snapshot.verify_source_unchanged().ok() != Some(true) {
        return Err("gradle_inputs_changed_during_projection");
    }
    Ok((findings, blockers))
}
/// 校验局部原生报告的封闭字段、规则和状态，返回协议错误；不授予规则完整性。
pub(crate) fn valid_native(v: &Value) -> Result<(), &'static str> {
    let schema_bytes = match v["schema_version"].as_str() {
        Some("0.1.0") => include_str!("../../../schemas/gradle-javadoc-probe-v0.1.schema.json"),
        Some("0.2.0") => include_str!("../../../schemas/gradle-javadoc-probe-v0.2.schema.json"),
        _ => return Err("gradle_native_identity_invalid"),
    };
    let schema: Value = serde_json::from_str(schema_bytes).expect("内置原生协议JSON有效");
    let keys = schema["required"].as_array().expect("内置字段列表");
    if !v.as_object().is_some_and(|o| {
        o.len() == keys.len()
            && keys
                .iter()
                .all(|k| o.contains_key(k.as_str().expect("字段为字符串")))
    }) || !matches!(v["schema_version"].as_str(), Some("0.1.0" | "0.2.0"))
        || v["report_type"] != "gradle_javadoc_probe"
        || v["coverage_proven"] != false
        || v["rule_configuration_complete"] != false
        || v["authority"] != "local_unverified"
        || v["delivery_decision"] != "not_evaluated"
        || !schema["properties"]["reason"]["enum"]
            .as_array()
            .expect("原因枚举")
            .contains(&v["reason"])
        || !matches!(
            v["native_status"].as_str(),
            Some("incomplete" | "findings_observed_unverified" | "empty_output_unverified")
        )
        || !v["selected_java_file_count"]
            .as_u64()
            .is_some_and(|n| n <= 128)
    {
        return Err("gradle_native_identity_invalid");
    }
    if !v["init_scripts_sha256"].as_str().is_some_and(sha) {
        return Err("gradle_native_digest_invalid");
    }
    let observed = v["native_status"] != "incomplete";
    for key in [
        "model_report_sha256",
        "source_snapshot_sha256",
        "gradle_bundle_sha256",
        "java_entry_sha256",
        "jdk_release_sha256",
        "native_stdout_sha256",
        "native_stderr_sha256",
    ] {
        if !(v[key].as_str().is_some_and(sha) || (!observed && v[key].is_null())) {
            return Err("gradle_native_digest_invalid");
        }
    }
    let tasks = v["task_paths"]
        .as_array()
        .filter(|r| r.len() <= 128)
        .ok_or("gradle_native_tasks_invalid")?;
    let mut seen = BTreeSet::new();
    for task in tasks {
        let task = task
            .as_str()
            .filter(|s| {
                s.len() <= 1280
                    && s.starts_with(':')
                    && !s.contains(['/', '\\'])
                    && !s.chars().any(char::is_control)
                    && s[1..].split(':').all(|p| !p.is_empty())
            })
            .ok_or("gradle_native_task_invalid")?;
        if !seen.insert(task) {
            return Err("gradle_native_task_duplicate");
        }
    }
    let findings = v["findings"]
        .as_array()
        .filter(|r| r.len() <= 10000)
        .ok_or("gradle_native_findings_invalid")?;
    for f in findings {
        if !exact(f, &["path", "rule_id", "line", "column"])
            || !f["path"]
                .as_str()
                .is_some_and(|p| safe(p) && p.ends_with(".java"))
            || !schema["properties"]["findings"]["items"]["properties"]["rule_id"]["enum"]
                .as_array()
                .expect("原生规则枚举")
                .contains(&f["rule_id"])
            || !f["line"].as_u64().is_some_and(|n| n > 0)
            || !f["column"].as_u64().is_some_and(|n| n > 0)
        {
            return Err("gradle_native_finding_invalid");
        }
    }
    if observed {
        if v["reason"] != "selected_sources_and_complete_documentation_rules_unverified"
            || tasks.is_empty()
            || v["selected_java_file_count"] == 0
            || (v["native_status"] == "findings_observed_unverified" && findings.is_empty())
            || (v["native_status"] == "empty_output_unverified" && !findings.is_empty())
        {
            return Err("gradle_native_status_invalid");
        }
    } else if !findings.is_empty()
        || v["reason"] == "selected_sources_and_complete_documentation_rules_unverified"
    {
        return Err("gradle_native_status_invalid");
    }
    Ok(())
}
fn safe(p: &str) -> bool {
    !p.is_empty()
        && p.len() <= 4096
        && !p.contains(['\\', ':'])
        && !p.chars().any(char::is_control)
        && !Path::new(p).is_absolute()
        && p.split('/').all(|s| !s.is_empty() && s != "." && s != "..")
}
fn sha(s: &str) -> bool {
    s.len() == 64
        && s.bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}
fn exact(v: &Value, keys: &[&str]) -> bool {
    v.as_object()
        .is_some_and(|o| o.len() == keys.len() && keys.iter().all(|k| o.contains_key(*k)))
}
fn digest(b: &[u8]) -> String {
    format!("{:x}", Sha256::digest(b))
}
fn identity(parts: &[&[u8]]) -> String {
    let mut h = Sha256::new();
    for p in parts {
        h.update((p.len() as u64).to_be_bytes());
        h.update(p);
    }
    format!("{:x}", h.finalize())
}

/// 绑定已初始化工作区并生成可持久化报告；参数为执行前选定输入和局部原生报告，返回未受信观察。
/// 不隐式初始化，不读取或执行报告中的命令，不授予关闭或完整覆盖。
pub fn prepare(root: &Path, inputs: &Value, native: &Value) -> Result<Value, &'static str> {
    let baseline =
        crate::workspace_refresh::read_workspace_baseline(root).map_err(|_| "workspace_invalid")?;
    let id = baseline
        .as_ref()
        .and_then(|b| b.workspace_id())
        .ok_or("workspace_not_initialized")?;
    let (findings, blockers) = project(root, inputs, native)?;
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|_| "clock_unavailable")?
        .as_nanos();
    Ok(
        json!({"schema_version":native["schema_version"],"report_type":"gradle_javadoc_workbench_observation","workspace_binding":"bound","workspace_id":id,"run_id":format!("javadoc-gradle-{}-{nanos}",std::process::id()),"checker_id":"java.gradle.javadoc","authority":"local_unverified","coverage_proven":false,"delivery_decision":"not_evaluated","inputs":inputs,"native":native,"findings":findings,"blockers":blockers}),
    )
}

/// 核对原报告后构造原任务复检参数；参数为工作区、任务ID、首次run和摘要，不执行工具。
pub(crate) fn recheck_argv(
    root: &Path,
    task_id: &str,
    run: &str,
    expected: &str,
) -> Result<Value, &'static str> {
    let bytes = codeguard_runtime::read_bounded_regular_file(
        &root.join(format!(".codeguard/reports/{run}.json")),
        16 * 1024 * 1024,
    )
    .map_err(|_| "gradle_original_report_unavailable")?;
    if digest(&bytes) != expected {
        return Err("gradle_original_report_changed");
    }
    let mut report = codeguard_adapters::parse_unique_json(&bytes)
        .map_err(|_| "gradle_original_report_invalid")?;
    if report["report_type"] == "gradle_javadoc_task_recheck" {
        if !crate::gradle_javadoc_task_recheck::valid_shape(&report) {
            return Err("gradle_original_report_invalid");
        }
        report = report["scan"].clone();
    }
    let baseline =
        crate::workspace_refresh::read_workspace_baseline(root).map_err(|_| "workspace_invalid")?;
    if report["report_type"] != "gradle_javadoc_workbench_observation"
        || report["run_id"] != run
        || report["workspace_id"].as_str() != baseline.as_ref().and_then(|b| b.workspace_id())
        || report["checker_id"] != "java.gradle.javadoc"
    {
        return Err("gradle_original_report_identity_invalid");
    }
    let rows = report["inputs"]
        .as_array()
        .filter(|r| !r.is_empty() && r.len() <= 128)
        .ok_or("gradle_original_inputs_invalid")?;
    for row in rows {
        if !row["path"].as_str().is_some_and(safe) {
            return Err("gradle_original_input_invalid");
        }
    }
    Ok(json!([
        "codeguard",
        "task",
        "verify",
        task_id,
        ".",
        "--gradle-bundle",
        "<已核验原Gradle绝对路径>",
        "--java-home",
        "<已核验原JDK21绝对路径>",
        "--format",
        "json"
    ]))
}

/// 保存和同步扫描前快照绑定的局部观察；返回更新计数，未初始化时不自动建工作区。
pub(crate) fn connect(root: &Path, snapshot: Option<&SourceSnapshot>, native: &Value) -> Value {
    let result: Result<Value, &'static str> = (|| {
        let snapshot = snapshot.ok_or("gradle_workbench_snapshot_unavailable")?;
        if snapshot.verify_source_unchanged().ok() != Some(true) {
            return Err("gradle_workbench_inputs_changed");
        }
        let inputs = json!(
            snapshot
                .files()
                .iter()
                .map(|(p, b)| json!({"path":p,"sha256":digest(b)}))
                .collect::<Vec<_>>()
        );
        let report = prepare(root, &inputs, native)?;
        crate::work_sync::save_local_report(root, &report)?;
        let summary = crate::work_sync::sync_local_workspace(root)?;
        Ok(
            json!({"status":if summary.failed_reports==0 {"synced_partial"} else {"sync_incomplete"},"new_findings":summary.new_findings,"new_blockers":summary.new_blockers,"task_verify_status":"local_observation_only","summary_scope":"workspace_sync"}),
        )
    })();
    result.unwrap_or_else(|reason|json!({"status":reason,"new_findings":0,"new_blockers":0,"task_verify_status":"local_observation_only","summary_scope":"workspace_sync"}))
}

/// 核对消费收据、原报告字节和准备观察，返回最新诊断及证据引用；不沿用未消费或篡改记录。
pub(crate) fn latest_preparation(
    root: &Path,
    id: &str,
    fact: &Value,
) -> Result<(String, Value), &'static str> {
    let directory = root.join(format!(".codeguard/state/observations/{id}"));
    if directory.canonicalize().ok().as_ref() != Some(&directory) {
        return Err("gradle_preparation_path_invalid");
    }
    let entries = std::fs::read_dir(&directory)
        .map_err(|_| "gradle_preparation_unavailable")?
        .take(1001)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| "gradle_preparation_unavailable")?;
    if entries.len() > 1000 {
        return Err("gradle_preparation_budget_exceeded");
    }
    let mut ordered = Vec::new();
    for entry in entries {
        let path = entry.path();
        let run = path
            .file_stem()
            .and_then(|s| s.to_str())
            .ok_or("gradle_preparation_run_invalid")?;
        if path.extension().and_then(|s| s.to_str()) != Some("json")
            || !run.starts_with("javadoc-gradle-")
            || !run.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-')
        {
            return Err("gradle_preparation_run_invalid");
        }
        let sequence = run
            .rsplit('-')
            .next()
            .and_then(|s| s.parse::<u128>().ok())
            .ok_or("gradle_preparation_run_invalid")?;
        ordered.push((sequence, path));
    }
    // 先按运行序列选最新已消费观察，只读取该报告，不重复载入全部历史大报告。
    ordered.sort_by_key(|a| std::cmp::Reverse(a.0));
    for (_, path) in ordered {
        let run = path
            .file_stem()
            .and_then(|s| s.to_str())
            .ok_or("gradle_preparation_run_invalid")?;
        let marker = root.join(format!(".codeguard/state/consumed/{run}.json"));
        if !marker
            .try_exists()
            .map_err(|_| "gradle_preparation_receipt_unavailable")?
        {
            continue;
        }
        let data = codeguard_runtime::read_bounded_regular_file(&path, 128 * 1024)
            .map_err(|_| "gradle_preparation_unavailable")?;
        let obs = codeguard_adapters::parse_unique_json(&data)
            .map_err(|_| "gradle_preparation_invalid")?;
        if !exact(
            &obs,
            &[
                "schema_version",
                "record_type",
                "blocker_id",
                "workspace_id",
                "run_id",
                "report_sha256",
                "reason_code",
                "affected_paths",
                "authority",
                "diagnostic_reason",
            ],
        ) || !matches!(obs["schema_version"].as_str(), Some("0.1.0" | "0.2.0"))
            || obs["record_type"] != "local_blocker_observation"
            || obs["blocker_id"] != id
            || obs["workspace_id"] != fact["workspace_id"]
            || obs["run_id"] != run
            || obs["reason_code"] != "gradle_javadoc_preparation_required"
            || obs["authority"] != "local_unverified"
        {
            return Err("gradle_preparation_identity_invalid");
        }
        let expected = obs["report_sha256"]
            .as_str()
            .filter(|s| sha(s))
            .ok_or("gradle_preparation_digest_invalid")?;
        let receipt = codeguard_runtime::read_bounded_regular_file(&marker, 4096)
            .map_err(|_| "gradle_preparation_receipt_unavailable")?;
        let wanted=serde_json::to_vec_pretty(&json!({"schema_version":"0.1.0","workspace_id":fact["workspace_id"],"run_id":run,"report_sha256":expected})).map_err(|_|"gradle_preparation_encoding_invalid")?;
        if receipt != wanted {
            return Err("gradle_preparation_receipt_invalid");
        }
        let report_bytes = codeguard_runtime::read_bounded_regular_file(
            &root.join(format!(".codeguard/reports/{run}.json")),
            16 * 1024 * 1024,
        )
        .map_err(|_| "gradle_preparation_report_unavailable")?;
        if digest(&report_bytes) != expected {
            return Err("gradle_preparation_report_changed");
        }
        let mut report = codeguard_adapters::parse_unique_json(&report_bytes)
            .map_err(|_| "gradle_preparation_report_invalid")?;
        if report["report_type"] == "gradle_javadoc_task_recheck" {
            if !crate::gradle_javadoc_task_recheck::valid_shape(&report) {
                return Err("gradle_preparation_report_invalid");
            }
            report = report["scan"].clone();
        }
        if report["report_type"] != "gradle_javadoc_workbench_observation"
            || report["workspace_id"] != fact["workspace_id"]
            || report["run_id"] != run
            || report["checker_id"] != "java.gradle.javadoc"
        {
            return Err("gradle_preparation_report_invalid");
        }
        valid_native(&report["native"])?;
        let blocker = report["blockers"]
            .as_array()
            .and_then(|rows| rows.iter().find(|b| b["id"] == id))
            .ok_or("gradle_preparation_report_invalid")?;
        if blocker["fingerprint"] != fact["fingerprint"]
            || blocker["affected_paths"] != obs["affected_paths"]
            || blocker["diagnostic_reason"] != obs["diagnostic_reason"]
            || obs["diagnostic_reason"] != report["native"]["reason"]
        {
            return Err("gradle_preparation_report_invalid");
        }
        return Ok((
            obs["diagnostic_reason"]
                .as_str()
                .ok_or("gradle_preparation_reason_invalid")?
                .to_owned(),
            json!({"run_id":run,"report_sha256":expected}),
        ));
    }
    Err("gradle_preparation_consumed_observation_missing")
}
