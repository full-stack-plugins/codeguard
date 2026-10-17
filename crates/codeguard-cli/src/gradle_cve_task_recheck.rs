//! 原Gradle CVE准备任务的冻结上下文复检；只保存未受信观察，不关闭任务。
use codeguard_runtime::{SourceSnapshot, read_bounded_regular_file};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeSet,
    path::{Path, PathBuf},
    sync::atomic::AtomicBool,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

/// 核对已消费的首次证据、工作区与稳定范围；参数是根与结构化任务，不信任Markdown。
pub(crate) fn original(root: &Path, brief: &Value) -> Result<Value, &'static str> {
    let run = brief["evidence_ref"]["first_run_id"]
        .as_str()
        .filter(|s| {
            s.starts_with("cve-gradle-")
                && s.len() <= 120
                && s.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-')
        })
        .ok_or("gradle_cve_original_run_invalid")?;
    let bytes = read_bounded_regular_file(
        &root.join(format!(".codeguard/reports/{run}.json")),
        128 * 1024,
    )
    .map_err(|_| "gradle_cve_original_unavailable")?;
    if brief["evidence_ref"]["first_report_sha256"] != hash(&bytes) {
        return Err("gradle_cve_original_changed");
    }
    let report =
        codeguard_adapters::parse_unique_json(&bytes).map_err(|_| "gradle_cve_original_invalid")?;
    crate::gradle_cve_workbench::validate(root, &report, false)?;
    let workspace = crate::workspace_refresh::read_workspace_baseline(root)
        .map_err(|_| "workspace_invalid")?
        .and_then(|b| b.workspace_id().map(str::to_owned))
        .ok_or("workspace_not_initialized")?;
    let receipt = read_bounded_regular_file(
        &root.join(format!(".codeguard/state/consumed/{run}.json")),
        4096,
    )
    .map_err(|_| "gradle_cve_original_receipt_unavailable")?;
    let expected=serde_json::to_vec_pretty(&json!({"schema_version":"0.1.0","workspace_id":workspace,"run_id":run,"report_sha256":hash(&bytes)})).map_err(|_|"gradle_cve_encoding_invalid")?;
    let fingerprint = crate::gradle_cve_workbench::fingerprint(&report)?;
    if receipt != expected
        || report["workspace_id"] != workspace
        || report["run_id"] != run
        || brief["checker_id"] != "java.gradle.dependency_check"
        || brief["kind"] != "blocker"
        || brief["scope"] != "."
        || brief["task_id"] != format!("CG-B-{}", &fingerprint[..32])
    {
        return Err("gradle_cve_original_binding_invalid");
    }
    Ok(report)
}
/// 按原工具、选定输入和原任务复检；未知原身份不提升资格，变化或前置缺失返回具体观察。
pub fn run(
    root: &Path,
    brief: &Value,
    bundle: Option<&Path>,
    java: Option<&Path>,
    cache: Option<&Path>,
    deadline: Instant,
    cancelled: &AtomicBool,
) -> Result<Value, &'static str> {
    let first = original(root, brief)?;
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| "clock_unavailable")?
        .as_nanos();
    let mut r = json!({"schema_version":"0.1.0","report_type":"gradle_cve_task_recheck","operation":"task_verify","run_id":format!("cve-gradle-task-{}-{nanos}",std::process::id()),"workspace_binding":"bound","workspace_id":first["workspace_id"],"checker_id":"java.gradle.dependency_check","authority":"local_unverified","coverage_proven":false,"delivery_decision":"not_evaluated","task_id":brief["task_id"],"origin":brief["evidence_ref"],"scan":null,"input_bindings":[],"task_input_stable":false,"original_inputs_match":false,"tool_identity_matches":false,"original_tool_identity_complete":false,"cache_identity_matches":false,"reason":"gradle_cve_recheck_incomplete"});
    macro_rules! stop {
        ($why:expr) => {{
            r["reason"] = json!($why);
            r["task_input_stable"] = json!(inputs_current(root, &r));
            return Ok(r);
        }};
    }
    if codeguard_runtime::sigint_cancellation_requested()
        || cancelled.load(std::sync::atomic::Ordering::Relaxed)
    {
        stop!("request_cancelled")
    }
    if Instant::now() >= deadline {
        stop!("request_deadline_exceeded")
    }
    let paths = first["inputs"]
        .as_array()
        .ok_or("gradle_cve_original_invalid")?
        .iter()
        .map(|p| {
            p["path"]
                .as_str()
                .map(PathBuf::from)
                .ok_or("gradle_cve_original_invalid")
        })
        .collect::<Result<BTreeSet<_>, _>>()?;
    let snapshot =
        match SourceSnapshot::capture(root, paths.clone(), 128, 1024 * 1024, 16 * 1024 * 1024) {
            Ok(s) => s,
            Err(_) => stop!("source_snapshot_unavailable"),
        };
    let rows: Vec<Value> = snapshot
        .files()
        .iter()
        .map(|(p, b)| json!({"path":p,"sha256":hash(b)}))
        .collect();
    let mut bindings: Vec<Value> = rows
        .iter()
        .map(|row| json!({"path":row["path"],"sha256":row["sha256"],"location":"workspace"}))
        .collect();
    r["input_bindings"] = json!(bindings);
    r["original_inputs_match"] = json!(first["inputs"] == json!(rows));
    if r["original_inputs_match"] != true {
        stop!("original_configuration_changed")
    }
    let scripts = hash(
        format!(
            "{}\n{}",
            include_str!("../resources/gradle_checker_model.init.gradle"),
            include_str!("../resources/gradle_dependency_check.init.gradle")
        )
        .as_bytes(),
    );
    if first["native_identity"]["init_scripts_sha256"] != scripts {
        stop!("original_init_scripts_changed")
    }
    if first["module_cache_selected"] != json!(cache.is_some()) {
        stop!("original_cache_selection_changed")
    }
    let (Some(bundle), Some(java)) = (bundle, java) else {
        stop!("prerequisites_missing")
    };
    if !bundle.is_absolute() || !java.is_absolute() || cache.is_some_and(|p| !p.is_absolute()) {
        stop!("native_tool_path_invalid")
    }
    let bundle_sha = match crate::tool_identity::hash_bundle_tree(bundle) {
        Ok(s) => s,
        Err(_) => stop!("native_tool_identity_unavailable"),
    };
    bindings.push(json!({"path":bundle,"sha256":bundle_sha,"location":"bundle"}));
    let mut identities = vec![bundle_sha];
    for path in [java.join("bin/java"), java.join("release")] {
        let bytes = match read_bounded_regular_file(&path, 128 * 1024 * 1024) {
            Ok(b) => b,
            Err(_) => stop!("native_tool_identity_unavailable"),
        };
        let sha = hash(&bytes);
        identities.push(sha.clone());
        bindings.push(json!({"path":path,"sha256":sha,"location":"tool"}));
    }
    r["input_bindings"] = json!(bindings);
    let keys = [
        "gradle_bundle_sha256",
        "java_entry_sha256",
        "jdk_release_sha256",
    ];
    r["original_tool_identity_complete"] = json!(
        keys.iter()
            .all(|k| first["native_identity"][*k].is_string())
    );
    r["tool_identity_matches"] = json!(
        keys.iter()
            .zip(&identities)
            .all(|(k, s)| first["native_identity"][*k].is_null()
                || first["native_identity"][*k] == *s)
    );
    if r["tool_identity_matches"] != true {
        stop!("original_tool_identity_mismatch")
    }
    if let Some(cache) = cache {
        let sha = match cache_hash(cache, deadline, cancelled) {
            Ok(s) => s,
            Err(reason) => stop!(reason),
        };
        bindings.push(json!({"path":cache,"sha256":sha,"location":"cache"}));
        r["input_bindings"] = json!(bindings);
        if !first["native_identity"]["dependency_cache_sha256"].is_null()
            && first["native_identity"]["dependency_cache_sha256"] != sha
        {
            stop!("original_cache_identity_mismatch")
        }
    }
    r["cache_identity_matches"] = json!(true);
    if Instant::now() >= deadline {
        stop!("request_deadline_exceeded")
    }
    if snapshot.verify_source_unchanged().ok() != Some(true) {
        stop!("selected_inputs_changed_before_scan")
    }
    let tasks = first["task_paths"]
        .as_array()
        .ok_or("gradle_cve_original_invalid")?
        .iter()
        .map(|t| {
            t.as_str()
                .map(str::to_owned)
                .ok_or("gradle_cve_original_invalid")
        })
        .collect::<Result<Vec<_>, _>>()?;
    let native = crate::gradle_dependency_check_probe::observe(
        &crate::gradle_dependency_check_request::Request {
            module_cache: cache.map(Path::to_owned),
            native: crate::gradle_model_probe_request::Request {
                project_root: root.to_owned(),
                project_files: paths.clone(),
                gradle_bundle: bundle.to_owned(),
                java_home: java.to_owned(),
                deadline,
            },
            task_paths: tasks.clone(),
        },
        cancelled,
    );
    let mut scan =
        crate::gradle_cve_workbench::prepare(root, &paths, &tasks, cache.is_some(), &native)?
            .ok_or("workspace_not_initialized")?;
    scan["run_id"] = r["run_id"].clone();
    r["scan"] = scan;
    r["reason"] = json!("gradle_cve_local_coverage_unverified");
    r["task_input_stable"] =
        json!(snapshot.verify_source_unchanged().ok() == Some(true) && inputs_current(root, &r));
    Ok(r)
}
/// 重算当前输入和显式工具/缓存的绑定；只读摘要，不执行报告内路径。
pub fn inputs_current(root: &Path, r: &Value) -> bool {
    let Some(rows) = r["input_bindings"]
        .as_array()
        .filter(|a| !a.is_empty() && a.len() <= 132)
    else {
        return false;
    };
    rows.iter().all(|row| {
        let Some(path) = row["path"].as_str() else {
            return false;
        };
        let p = Path::new(path);
        match row["location"].as_str() {
            Some("workspace") => {
                crate::gradle_cve_workbench::safe_input_path(path)
                    && read_bounded_regular_file(&root.join(p), 1024 * 1024)
                        .ok()
                        .is_some_and(|b| row["sha256"] == hash(&b))
            }
            Some("bundle") => {
                p.is_absolute()
                    && crate::tool_identity::hash_bundle_tree(p)
                        .ok()
                        .is_some_and(|s| row["sha256"] == s)
            }
            Some("tool") => {
                p.is_absolute()
                    && read_bounded_regular_file(p, 128 * 1024 * 1024)
                        .ok()
                        .is_some_and(|b| row["sha256"] == hash(&b))
            }
            Some("cache") => {
                p.is_absolute()
                    && cache_hash(
                        p,
                        Instant::now() + Duration::from_secs(90),
                        &AtomicBool::new(false),
                    )
                    .ok()
                    .is_some_and(|s| row["sha256"] == s)
            }
            _ => false,
        }
    })
}
/// 验证封闭局部容器；完整来源绑定由validate_binding进一步核对，不授予关闭资格。
pub fn valid_shape(r: &Value) -> bool {
    let keys = [
        "schema_version",
        "report_type",
        "operation",
        "run_id",
        "workspace_binding",
        "workspace_id",
        "checker_id",
        "authority",
        "coverage_proven",
        "delivery_decision",
        "task_id",
        "origin",
        "scan",
        "input_bindings",
        "task_input_stable",
        "original_inputs_match",
        "tool_identity_matches",
        "original_tool_identity_complete",
        "cache_identity_matches",
        "reason",
    ];
    r.as_object()
        .is_some_and(|o| o.len() == keys.len() && keys.iter().all(|k| o.contains_key(*k)))
        && r["schema_version"] == "0.1.0"
        && r["report_type"] == "gradle_cve_task_recheck"
        && r["operation"] == "task_verify"
        && r["workspace_binding"] == "bound"
        && r["checker_id"] == "java.gradle.dependency_check"
        && r["authority"] == "local_unverified"
        && r["coverage_proven"] == false
        && r["delivery_decision"] == "not_evaluated"
        && r["task_id"]
            .as_str()
            .is_some_and(crate::next_command::safe_id)
        && r["reason"].as_str().is_some_and(|s| {
            s.len() <= 128 && s.bytes().all(|b| b.is_ascii_lowercase() || b == b'_')
        })
        && r["run_id"].as_str().is_some_and(|s| {
            s.starts_with("cve-gradle-task-")
                && s.len() <= 120
                && s.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-')
        })
        && [
            "task_input_stable",
            "original_inputs_match",
            "tool_identity_matches",
            "original_tool_identity_complete",
            "cache_identity_matches",
        ]
        .iter()
        .all(|k| r[*k].is_boolean())
        && r["input_bindings"].as_array().is_some_and(|a| {
            a.len() <= 132
                && a.iter().all(|row| {
                    row.as_object().is_some_and(|o| {
                        o.len() == 3
                            && o.contains_key("path")
                            && o.contains_key("sha256")
                            && o.contains_key("location")
                    }) && row["path"]
                        .as_str()
                        .is_some_and(|p| match row["location"].as_str() {
                            Some("workspace") => crate::gradle_cve_workbench::safe_input_path(p),
                            Some("bundle" | "tool" | "cache") => Path::new(p).is_absolute(),
                            _ => false,
                        })
                        && row["sha256"].as_str().is_some_and(sha)
                })
        })
        && (r["scan"].is_null() || r["scan"].is_object())
}
/// 重核原收据和内层脱敏投影，不信任外层输入/工具匹配布尔值。
pub(crate) fn validate_binding(root: &Path, brief: &Value, r: &Value) -> Result<(), &'static str> {
    if !valid_shape(r) || r["task_id"] != brief["task_id"] || r["origin"] != brief["evidence_ref"] {
        return Err("gradle_cve_recheck_invalid");
    }
    let first = original(root, brief)?;
    if r["workspace_id"] != first["workspace_id"] {
        return Err("gradle_cve_recheck_workspace_invalid");
    }
    if r["scan"].is_null() {
        return Ok(());
    }
    let complete = [
        "gradle_bundle_sha256",
        "java_entry_sha256",
        "jdk_release_sha256",
    ]
    .iter()
    .all(|k| first["native_identity"][*k].is_string());
    if r["original_tool_identity_complete"] != json!(complete) {
        return Err("gradle_cve_recheck_identity_invalid");
    }
    let scan = &r["scan"];
    crate::gradle_cve_workbench::validate(root, scan, false)?;
    if scan["workspace_id"] != r["workspace_id"]
        || scan["run_id"] != r["run_id"]
        || scan["inputs"] != first["inputs"]
        || scan["task_paths"] != first["task_paths"]
        || scan["module_cache_selected"] != first["module_cache_selected"]
        || r["original_inputs_match"] != true
        || r["tool_identity_matches"] != true
        || r["cache_identity_matches"] != true
    {
        return Err("gradle_cve_recheck_scope_invalid");
    }
    // 输入绑定必须完整对应内层扫描，不能删减文件或替换外部摘要来掩盖变化。
    let bindings = r["input_bindings"]
        .as_array()
        .ok_or("gradle_cve_recheck_scope_invalid")?;
    let workspace_rows: Vec<Value> = bindings
        .iter()
        .filter(|row| row["location"] == "workspace")
        .map(|row| json!({"path":row["path"],"sha256":row["sha256"]}))
        .collect();
    if json!(workspace_rows) != scan["inputs"] {
        return Err("gradle_cve_recheck_scope_invalid");
    }
    for (location, key, count) in [
        ("bundle", "gradle_bundle_sha256", 1),
        (
            "cache",
            "dependency_cache_sha256",
            usize::from(first["module_cache_selected"] == true),
        ),
    ] {
        let rows: Vec<&Value> = bindings
            .iter()
            .filter(|row| row["location"] == location)
            .collect();
        if rows.len() != count
            || rows
                .iter()
                .any(|row| row["sha256"] != scan["native_identity"][key])
        {
            return Err("gradle_cve_recheck_identity_invalid");
        }
    }
    let tools: Vec<&Value> = bindings
        .iter()
        .filter(|row| row["location"] == "tool")
        .collect();
    if tools.len() != 2
        || tools[0]["sha256"] != scan["native_identity"]["java_entry_sha256"]
        || tools[1]["sha256"] != scan["native_identity"]["jdk_release_sha256"]
    {
        return Err("gradle_cve_recheck_identity_invalid");
    }
    for k in [
        "gradle_bundle_sha256",
        "java_entry_sha256",
        "jdk_release_sha256",
        "dependency_cache_sha256",
        "init_scripts_sha256",
    ] {
        if !first["native_identity"][k].is_null()
            && scan["native_identity"][k] != first["native_identity"][k]
        {
            return Err("gradle_cve_recheck_identity_invalid");
        }
    }
    Ok(())
}
/// 分类始终保留开放准备任务；局部零漏洞与原身份未知均不能关闭。
pub fn classify(brief: &Value, r: &Value) -> &'static str {
    if !valid_shape(r)
        || r["task_id"] != brief["task_id"]
        || r["origin"] != brief["evidence_ref"]
        || r["task_input_stable"] != true
    {
        "incomplete"
    } else if r["reason"] == "original_configuration_changed"
        || (r["original_tool_identity_complete"] != true && r["scan"].is_object())
    {
        "rule_coverage_requires_review"
    } else if r["scan"]["native_status"] == "reports_observed_unverified" {
        "still_blocked"
    } else {
        "incomplete"
    }
}
fn cache_hash(
    path: &Path,
    deadline: Instant,
    cancelled: &AtomicBool,
) -> Result<String, &'static str> {
    let snapshot = crate::gradle_module_cache::capture(path, deadline, cancelled)?;
    let rows: Vec<Value> = snapshot
        .files()
        .iter()
        .map(|(p, b)| json!({"path":p,"sha256":hash(b)}))
        .collect();
    Ok(hash(
        &serde_json::to_vec(&rows).map_err(|_| "gradle_cve_encoding_invalid")?,
    ))
}
fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn sha(s: &str) -> bool {
    s.len() == 64
        && s.bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}
