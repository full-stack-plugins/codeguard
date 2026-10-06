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
fn valid_native(v: &Value) -> Result<(), &'static str> {
    let schema: Value = serde_json::from_str(include_str!(
        "../../../schemas/gradle-javadoc-probe-v0.1.schema.json"
    ))
    .expect("内置原生协议JSON有效");
    let keys = schema["required"].as_array().expect("内置字段列表");
    if !v.as_object().is_some_and(|o| {
        o.len() == keys.len()
            && keys
                .iter()
                .all(|k| o.contains_key(k.as_str().expect("字段为字符串")))
    }) || v["schema_version"] != "0.1.0"
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
