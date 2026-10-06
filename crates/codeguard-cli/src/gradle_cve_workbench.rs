//! 显式Gradle CVE到脱敏稳定准备任务；原生观察不能授予漏洞或关闭资格。
use codeguard_runtime::SourceSnapshot;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeSet,
    path::{Component, Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};
static NEXT: AtomicU64 = AtomicU64::new(0);

/// 构造已绑定当前输入的脱敏观察；未初始化返回None，不在此函数写工作区。
/// 参数为项目根、原选定输入及任务、可选缓存选择和本轮原生报告；返回待同步观察。
pub(crate) fn prepare(
    root: &Path,
    paths: &BTreeSet<PathBuf>,
    tasks: &[String],
    cache: bool,
    native: &Value,
) -> Result<Option<Value>, &'static str> {
    if codeguard_runtime::sigint_cancellation_requested() {
        return Err("request_cancelled");
    }
    let baseline = crate::workspace_refresh::read_workspace_baseline(root)
        .map_err(|_| "gradle_cve_workspace_invalid")?;
    let Some(workspace) = baseline.and_then(|b| b.workspace_id().map(str::to_owned)) else {
        return Ok(None);
    };
    let snapshot = SourceSnapshot::capture(
        root,
        paths.iter().cloned(),
        128,
        1024 * 1024,
        16 * 1024 * 1024,
    )
    .map_err(|_| "gradle_cve_inputs_unavailable")?;
    let inputs: Vec<Value> = snapshot
        .files()
        .iter()
        .map(|(p, b)| json!({"path":p,"sha256":hash(b)}))
        .collect();
    let source = hash(&serde_json::to_vec(&inputs).map_err(|_| "gradle_cve_encoding_invalid")?);
    if !native["source_snapshot_sha256"].is_null() && native["source_snapshot_sha256"] != source {
        return Err("gradle_cve_inputs_changed_after_scan");
    }
    let summaries: Vec<Value> = native["reports"].as_array().ok_or("gradle_cve_native_invalid")?.iter().map(|r|json!({"task_path":r["task_path"],"report_sha256":r["report_sha256"],"dependency_count":r["dependency_count"],"advisory_count":r["advisories"].as_array().map_or(0,Vec::len)})).collect();
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| "clock_unavailable")?
        .as_nanos();
    let mut native_identity = serde_json::Map::new();
    for key in [
        "source_snapshot_sha256",
        "gradle_bundle_sha256",
        "java_entry_sha256",
        "jdk_release_sha256",
        "dependency_cache_sha256",
        "model_report_sha256",
        "ownership_report_sha256",
        "init_scripts_sha256",
    ] {
        native_identity.insert(key.into(), native[key].clone());
    }
    let report = json!({"schema_version":"0.1.0","report_type":"gradle_cve_workbench_observation","workspace_binding":"bound","workspace_id":workspace,"run_id":format!("cve-gradle-{}-{nonce}-{}",std::process::id(),NEXT.fetch_add(1,Ordering::Relaxed)),"checker_id":"java.gradle.dependency_check","authority":"local_unverified","coverage_proven":false,"delivery_decision":"not_evaluated","native_identity":native_identity,"inputs":inputs,"task_paths":tasks,"module_cache_selected":cache,"native_status":native["native_status"],"diagnostic_reason":native["reason"],"native_report_sha256":hash(&serde_json::to_vec(native).map_err(|_|"gradle_cve_encoding_invalid")?),"report_summaries":summaries});
    validate(root, &report, true)?;
    if snapshot.verify_source_unchanged().ok() != Some(true) {
        return Err("gradle_cve_inputs_changed_after_scan");
    }
    if codeguard_runtime::sigint_cancellation_requested() {
        return Err("request_cancelled");
    }
    Ok(Some(report))
}

/// 检查脱敏观察的封闭协议及当前输入；历史查询可保留原始输入，导入必须核对当前字节。
pub(crate) fn validate(root: &Path, v: &Value, current: bool) -> Result<(), &'static str> {
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
        "native_identity",
        "inputs",
        "task_paths",
        "module_cache_selected",
        "native_status",
        "diagnostic_reason",
        "native_report_sha256",
        "report_summaries",
    ];
    if !exact(v, &keys)
        || v["schema_version"] != "0.1.0"
        || v["report_type"] != "gradle_cve_workbench_observation"
        || v["workspace_binding"] != "bound"
        || v["checker_id"] != "java.gradle.dependency_check"
        || v["authority"] != "local_unverified"
        || v["coverage_proven"] != false
        || v["delivery_decision"] != "not_evaluated"
        || !v["module_cache_selected"].is_boolean()
        || !sha(&v["native_report_sha256"])
    {
        return Err("gradle_cve_report_identity_invalid");
    }
    let identity_keys = [
        "source_snapshot_sha256",
        "gradle_bundle_sha256",
        "java_entry_sha256",
        "jdk_release_sha256",
        "dependency_cache_sha256",
        "model_report_sha256",
        "ownership_report_sha256",
        "init_scripts_sha256",
    ];
    if !exact(&v["native_identity"], &identity_keys)
        || identity_keys
            .iter()
            .any(|k| !(v["native_identity"][*k].is_null() || sha(&v["native_identity"][*k])))
        || !sha(&v["native_identity"]["init_scripts_sha256"])
    {
        return Err("gradle_cve_native_identity_invalid");
    }
    let expected =
        hash(&serde_json::to_vec(&v["inputs"]).map_err(|_| "gradle_cve_encoding_invalid")?);
    if !v["native_identity"]["source_snapshot_sha256"].is_null()
        && v["native_identity"]["source_snapshot_sha256"] != expected
    {
        return Err("gradle_cve_native_source_invalid");
    }
    if v["native_status"] == "reports_observed_unverified"
        && identity_keys
            .iter()
            .filter(|k| **k != "dependency_cache_sha256")
            .any(|k| !sha(&v["native_identity"][*k]))
    {
        return Err("gradle_cve_native_identity_invalid");
    }
    let rows = v["inputs"]
        .as_array()
        .filter(|r| !r.is_empty() && r.len() <= 128)
        .ok_or("gradle_cve_inputs_invalid")?;
    let mut paths = BTreeSet::new();
    for r in rows {
        let p = r["path"]
            .as_str()
            .filter(|s| safe_path(s))
            .ok_or("gradle_cve_input_invalid")?;
        if !exact(r, &["path", "sha256"]) || !sha(&r["sha256"]) || !paths.insert(PathBuf::from(p)) {
            return Err("gradle_cve_input_invalid");
        }
    }
    if !["settings.gradle", "settings.gradle.kts"]
        .iter()
        .any(|s| paths.contains(Path::new(s)))
        || !["build.gradle", "build.gradle.kts"]
            .iter()
            .any(|s| paths.contains(Path::new(s)))
    {
        return Err("gradle_cve_root_inputs_missing");
    }
    if current {
        let snapshot = SourceSnapshot::capture(root, paths, 128, 1024 * 1024, 16 * 1024 * 1024)
            .map_err(|_| "gradle_cve_inputs_unavailable")?;
        let actual: Vec<Value> = snapshot
            .files()
            .iter()
            .map(|(p, b)| json!({"path":p,"sha256":hash(b)}))
            .collect();
        if v["inputs"] != json!(actual) || snapshot.verify_source_unchanged().ok() != Some(true) {
            return Err("gradle_cve_inputs_stale");
        }
    }
    let tasks = v["task_paths"]
        .as_array()
        .filter(|r| !r.is_empty() && r.len() <= 128)
        .ok_or("gradle_cve_tasks_invalid")?;
    let mut seen = BTreeSet::new();
    for t in tasks {
        if !t.as_str().is_some_and(safe_task)
            || !seen.insert(t.as_str().ok_or("gradle_cve_tasks_invalid")?)
        {
            return Err("gradle_cve_tasks_invalid");
        }
    }
    let reason_schema: Value = serde_json::from_str(include_str!(
        "../../../schemas/gradle-dependency-check-probe-v0.2.schema.json"
    ))
    .expect("原生协议有效");
    if !reason_schema["properties"]["reason"]["enum"]
        .as_array()
        .expect("原因枚举")
        .contains(&v["diagnostic_reason"])
    {
        return Err("gradle_cve_reason_invalid");
    }
    let rows = v["report_summaries"]
        .as_array()
        .filter(|r| r.len() <= 128)
        .ok_or("gradle_cve_summary_invalid")?;
    let observed = v["native_status"] == "reports_observed_unverified";
    let observed_reason = matches!(
        v["diagnostic_reason"].as_str(),
        Some(
            "database_and_dependency_coverage_unverified"
                | "native_failure_with_reports_unverified"
        )
    );
    if observed != observed_reason
        || (observed
            && v["module_cache_selected"] == true
            && !sha(&v["native_identity"]["dependency_cache_sha256"]))
        || (v["module_cache_selected"] == false
            && !v["native_identity"]["dependency_cache_sha256"].is_null())
    {
        return Err("gradle_cve_native_state_invalid");
    }
    if (!observed && v["native_status"] != "incomplete")
        || (!observed && !rows.is_empty())
        || (observed
            && (rows.len() != tasks.len()
                || !matches!(
                    v["diagnostic_reason"].as_str(),
                    Some(
                        "database_and_dependency_coverage_unverified"
                            | "native_failure_with_reports_unverified"
                    )
                )))
    {
        return Err("gradle_cve_summary_invalid");
    }
    let mut reports = BTreeSet::new();
    let mut count = 0;
    for row in rows {
        if !exact(
            row,
            &[
                "task_path",
                "report_sha256",
                "dependency_count",
                "advisory_count",
            ],
        ) || !tasks.contains(&row["task_path"])
            || !reports.insert(
                row["task_path"]
                    .as_str()
                    .ok_or("gradle_cve_summary_invalid")?,
            )
            || !sha(&row["report_sha256"])
            || !row["dependency_count"]
                .as_u64()
                .is_some_and(|n| n <= 100000)
        {
            return Err("gradle_cve_summary_invalid");
        }
        count += row["advisory_count"]
            .as_u64()
            .filter(|n| *n <= 1000)
            .ok_or("gradle_cve_summary_invalid")?;
        if count > 1000 {
            return Err("gradle_cve_summary_invalid");
        }
    }
    Ok(())
}
/// 稳定标识只绑定检查器、选定路径和原任务集合，诊断/内容变化追加同一准备任务。
pub(crate) fn fingerprint(v: &Value) -> Result<String, &'static str> {
    let paths: Vec<&Value> = v["inputs"]
        .as_array()
        .ok_or("gradle_cve_inputs_invalid")?
        .iter()
        .map(|r| &r["path"])
        .collect();
    let mut tasks = v["task_paths"]
        .as_array()
        .ok_or("gradle_cve_tasks_invalid")?
        .clone();
    tasks.sort_by_key(|v| v.as_str().unwrap_or("").to_owned());
    Ok(hash(
        &serde_json::to_vec(&json!([
            "codeguard-gradle-cve-preparation-v1",
            paths,
            tasks
        ]))
        .map_err(|_| "gradle_cve_encoding_invalid")?,
    ))
}
fn exact(v: &Value, keys: &[&str]) -> bool {
    v.as_object()
        .is_some_and(|o| o.len() == keys.len() && keys.iter().all(|k| o.contains_key(*k)))
}
fn sha(v: &Value) -> bool {
    v.as_str().is_some_and(|s| {
        s.len() == 64
            && s.bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    })
}
fn safe_path(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 256
        && !s.contains(['\\', ':'])
        && !s.chars().any(char::is_control)
        && Path::new(s)
            .components()
            .all(|c| matches!(c, Component::Normal(_)))
}
fn safe_task(s: &str) -> bool {
    s.len() <= 256
        && s.starts_with(':')
        && s[1..].split(':').all(|p| {
            !p.is_empty()
                && p.bytes()
                    .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'-'))
        })
}
fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

/// 保存脱敏观察并自动同步已初始化工作区；参数和返回含义同prepare，未初始化返回false。
pub fn persist(
    root: &Path,
    paths: &BTreeSet<PathBuf>,
    tasks: &[String],
    cache: bool,
    native: &Value,
) -> Result<bool, &'static str> {
    let Some(report) = prepare(root, paths, tasks, cache, native)? else {
        return Ok(false);
    };
    crate::work_sync::save_local_report(root, &report)?;
    if crate::work_sync::sync_local_workspace(root)?.failed_reports != 0 {
        return Err("gradle_cve_sync_incomplete");
    }
    Ok(true)
}

/// 选定输入只接受根内普通相对路径；复检和首次导入共享同一边界。
pub(crate) fn safe_input_path(s: &str) -> bool {
    safe_path(s)
}
