//! 执行显式Gradle OWASP原任务并读取本轮唯一原生报告；不签发漏洞清洁结论。
use crate::{gradle_dependency_check_request::Request, tool_identity::hash_bundle_tree};
use codeguard_adapters::{
    parse_gradle_checker_model, parse_gradle_owasp_report_ownership,
    parse_owasp_dependency_check_json, plan_gradle_dependency_check_tasks,
};
use codeguard_runtime::{
    ProcessSpec, SourceSnapshot, Termination, read_bounded_regular_file, run_process_recorded,
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    ffi::OsString,
    fs,
    os::unix::fs::PermissionsExt,
    path::PathBuf,
    sync::atomic::{AtomicBool, AtomicU64, Ordering},
    time::{Instant, SystemTime, UNIX_EPOCH},
};
static NEXT: AtomicU64 = AtomicU64::new(0);
struct Scratch(PathBuf);
impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

const MODEL_SCRIPT: &str = include_str!("../resources/gradle_checker_model.init.gradle");
const OWASP_SCRIPT: &str = include_str!("../resources/gradle_dependency_check.init.gradle");
/// 在私有副本的一次Gradle调用中观察模型、执行显式OWASP原任务并解析当前报告。
/// 参数为已有工具、选定输入、原任务及取消信号；返回未验证数据库/依赖覆盖的局部观察。
pub fn observe(selected: &Request, cancelled: &AtomicBool) -> Value {
    let mut report = missing_prerequisites();
    if selected.task_paths.is_empty()
        || selected.task_paths.len() > 128
        || selected
            .task_paths
            .iter()
            .any(|path| !valid_task_path(path))
        || selected
            .task_paths
            .iter()
            .collect::<std::collections::BTreeSet<_>>()
            .len()
            != selected.task_paths.len()
    {
        report["reason"] = json!("gradle_owasp_task_selection_invalid");
        return report;
    }
    let request = &selected.native;
    macro_rules! fail {
        ($why:expr) => {{
            report["reason"] = json!($why);
            return report;
        }};
    }
    if cancelled.load(Ordering::Relaxed) || codeguard_runtime::sigint_cancellation_requested() {
        fail!("request_cancelled");
    }
    if Instant::now() >= request.deadline {
        fail!("request_deadline_exceeded");
    }
    if !["settings.gradle", "settings.gradle.kts"]
        .iter()
        .any(|p| request.project_files.contains(&PathBuf::from(p)))
        || !["build.gradle", "build.gradle.kts"]
            .iter()
            .any(|p| request.project_files.contains(&PathBuf::from(p)))
    {
        fail!("root_build_inputs_missing");
    }
    let Ok(snapshot) = SourceSnapshot::capture(
        &request.project_root,
        request.project_files.iter().cloned(),
        128,
        1024 * 1024,
        16 * 1024 * 1024,
    ) else {
        fail!("selected_inputs_unavailable");
    };
    let files = snapshot
        .files()
        .iter()
        .map(|(path, bytes)| json!({"path":path,"sha256":digest(bytes)}))
        .collect::<Vec<_>>();
    report["source_snapshot_sha256"] =
        json!(digest(&serde_json::to_vec(&files).expect("摘要可序列化")));
    let (Ok(bundle), Ok(java_home)) = (
        request.gradle_bundle.canonicalize(),
        request.java_home.canonicalize(),
    ) else {
        fail!("native_prerequisite_unavailable");
    };
    let java = java_home.join("bin/java");
    let release = java_home.join("release");
    let (Ok(bundle_hash), Ok(java_bytes), Ok(release_bytes)) = (
        hash_bundle_tree(&bundle),
        read_bounded_regular_file(&java, 128 * 1024 * 1024),
        read_bounded_regular_file(&release, 64 * 1024),
    ) else {
        fail!("native_identity_unavailable");
    };
    report["gradle_bundle_sha256"] = json!(bundle_hash);
    report["java_entry_sha256"] = json!(digest(&java_bytes));
    report["jdk_release_sha256"] = json!(digest(&release_bytes));
    let release_text = std::str::from_utf8(&release_bytes).unwrap_or("");
    if !release_text
        .lines()
        .any(|s| s.starts_with("JAVA_VERSION=\"21.") || s == "JAVA_VERSION=\"21\"")
    {
        fail!("jdk21_version_unresolved");
    }
    if Instant::now() >= request.deadline {
        fail!("request_deadline_exceeded");
    }
    let cache_snapshot = if let Some(cache) = &selected.module_cache {
        match crate::gradle_module_cache::capture(cache, request.deadline, cancelled) {
            Ok(snapshot) => {
                let identities = snapshot
                    .files()
                    .iter()
                    .map(|(path, bytes)| json!({"path":path,"sha256":digest(bytes)}))
                    .collect::<Vec<_>>();
                report["dependency_cache_sha256"] = json!(digest(
                    &serde_json::to_vec(&identities).expect("缓存摘要可序列化")
                ));
                Some(snapshot)
            }
            Err(reason) => {
                if cancelled.load(Ordering::Relaxed)
                    || codeguard_runtime::sigint_cancellation_requested()
                {
                    fail!("request_cancelled");
                }
                if Instant::now() >= request.deadline {
                    fail!("request_deadline_exceeded");
                }
                fail!(reason);
            }
        }
    } else {
        None
    };
    let Ok(scratch) = private_scratch() else {
        fail!("private_workspace_unavailable");
    };
    let project = scratch.0.join("project");
    let evidence = scratch.0.join("evidence");
    let home = scratch.0.join("home");
    let model_script = scratch.0.join("model.init.gradle");
    let owasp_script = scratch.0.join("owasp.init.gradle");
    let model_output = evidence.join("model.json");
    let ownership_output = evidence.join("ownership.json");
    if snapshot.materialize_new(&project).is_err()
        || fs::create_dir(&evidence).is_err()
        || fs::create_dir(&home).is_err()
        || fs::write(&model_script, MODEL_SCRIPT).is_err()
        || fs::write(&owasp_script, OWASP_SCRIPT).is_err()
        || fs::set_permissions(&evidence, fs::Permissions::from_mode(0o700)).is_err()
        || fs::set_permissions(&home, fs::Permissions::from_mode(0o700)).is_err()
    {
        fail!("private_workspace_unavailable");
    }
    if let Some(cache) = &cache_snapshot {
        let caches = home.join(".gradle/caches");
        if fs::create_dir_all(&caches).is_err()
            || cache.materialize_new(&caches.join("modules-2")).is_err()
        {
            fail!("gradle_dependency_cache_materialization_incomplete");
        }
    }
    let mut env = BTreeMap::new();
    env.insert(
        OsString::from("JAVA_HOME"),
        java_home.as_os_str().to_owned(),
    );
    env.insert(OsString::from("HOME"), home.as_os_str().to_owned());
    env.insert(OsString::from("PATH"), OsString::from("/usr/bin:/bin"));
    let mut args = vec![
        "--offline".into(),
        "--no-daemon".into(),
        "--console=plain".into(),
        "--no-configuration-cache".into(),
        "--rerun-tasks".into(),
        "--no-build-cache".into(),
        "--init-script".into(),
        model_script.clone().into_os_string(),
        "--init-script".into(),
        owasp_script.clone().into_os_string(),
        format!("-Dcodeguard.model.output={}", model_output.display()).into(),
        format!("-Dcodeguard.owasp.tasks={}", selected.task_paths.join(",")).into(),
        format!("-Dcodeguard.owasp.ownership={}", ownership_output.display()).into(),
        "--project-dir".into(),
        project.clone().into_os_string(),
        "--gradle-user-home".into(),
        home.join(".gradle").into_os_string(),
        format!("-Duser.home={}", home.display()).into(),
        "-Duser.language=en".into(),
        "-Duser.country=US".into(),
        "-Dorg.gradle.parallel=false".into(),
    ];
    args.push(":codeguardRunDependencyCheck".into());
    let spec = ProcessSpec {
        executable: bundle.join("bin/gradle"),
        args,
        cwd: project.clone(),
        env,
        stdin: None,
        deadline: request.deadline,
        output_limit_bytes: 2 * 1024 * 1024,
    };
    let outcome = run_process_recorded(&spec, cancelled, &evidence, "gradle-owasp.log");
    if cancelled.load(Ordering::Relaxed) || codeguard_runtime::sigint_cancellation_requested() {
        fail!("request_cancelled");
    }
    if Instant::now() >= request.deadline {
        fail!("request_deadline_exceeded");
    }
    let Ok(outcome) = outcome else {
        fail!("native_process_incomplete");
    };
    if snapshot.verify_unchanged(&project).ok() != Some(true)
        || read_bounded_regular_file(&model_script, 64 * 1024)
            .ok()
            .as_deref()
            != Some(MODEL_SCRIPT.as_bytes())
        || read_bounded_regular_file(&owasp_script, 64 * 1024)
            .ok()
            .as_deref()
            != Some(OWASP_SCRIPT.as_bytes())
    {
        fail!("selected_inputs_changed_during_task");
    }
    if hash_bundle_tree(&bundle).ok().as_deref() != Some(bundle_hash.as_str())
        || read_bounded_regular_file(&java, 128 * 1024 * 1024)
            .ok()
            .as_deref()
            != Some(java_bytes.as_slice())
        || read_bounded_regular_file(&release, 64 * 1024)
            .ok()
            .as_deref()
            != Some(release_bytes.as_slice())
    {
        fail!("native_identity_changed_during_task");
    }
    if cache_snapshot
        .as_ref()
        .is_some_and(|cache| cache.verify_source_unchanged().ok() != Some(true))
    {
        fail!("gradle_dependency_cache_changed_during_task");
    }
    report["native_stdout_sha256"] = json!(digest(&outcome.stdout));
    report["native_stderr_sha256"] = json!(digest(&outcome.stderr));
    let exit = match outcome.termination {
        Termination::Exited(value) if matches!(value, 0 | 1) => value,
        _ => fail!("native_execution_incomplete"),
    };
    report["native_exit_code"] = json!(exit);
    let Ok(model_bytes) = read_bounded_regular_file(&model_output, 1024 * 1024) else {
        fail!(
            native_configuration_blocker(&outcome.stdout, &outcome.stderr)
                .unwrap_or("gradle_native_model_incomplete")
        );
    };
    let Ok(model) = parse_gradle_checker_model(&model_bytes) else {
        fail!("gradle_native_model_invalid");
    };
    let Ok(plans) = plan_gradle_dependency_check_tasks(&model, &selected.task_paths) else {
        fail!("gradle_owasp_task_unavailable");
    };
    report["model_report_sha256"] = json!(digest(&model_bytes));
    report["task_paths"] = json!(
        plans
            .iter()
            .map(|p| p.task_path.as_str())
            .collect::<Vec<_>>()
    );
    let Ok(ownership_bytes) = read_bounded_regular_file(&ownership_output, 1024 * 1024) else {
        fail!(
            native_configuration_blocker(&outcome.stdout, &outcome.stderr)
                .unwrap_or("gradle_owasp_report_ownership_unavailable")
        );
    };
    let Ok(ownership) =
        parse_gradle_owasp_report_ownership(&ownership_bytes, &model, &selected.task_paths)
    else {
        fail!("gradle_owasp_report_ownership_invalid");
    };
    report["ownership_report_sha256"] = json!(digest(&ownership_bytes));
    let mut reports = Vec::new();
    let mut budget = crate::gradle_owasp_report_budget::GradleOwaspReportBudget::default();
    for owner in ownership {
        if cancelled.load(Ordering::Relaxed) || codeguard_runtime::sigint_cancellation_requested() {
            fail!("request_cancelled");
        }
        if Instant::now() >= request.deadline {
            fail!("request_deadline_exceeded");
        }

        // 原输出不能来自选定输入，且逐级物理路径不得通过符号链接逃逸私有副本。
        if snapshot
            .files()
            .contains_key(&PathBuf::from(&owner.report_path))
        {
            fail!("gradle_owasp_report_preexisting");
        }
        let path = project.join(&owner.report_path);
        let Ok(canonical) = path.canonicalize() else {
            fail!("gradle_owasp_report_missing");
        };
        if !canonical.starts_with(&project) || canonical != path {
            fail!("gradle_owasp_report_scope_invalid");
        }
        let limit = budget.remaining_input().min(8 * 1024 * 1024);
        if limit == 0 {
            fail!("gradle_owasp_report_budget_exceeded");
        }
        if fs::symlink_metadata(&path)
            .ok()
            .is_some_and(|metadata| metadata.len() > limit as u64)
        {
            fail!(if limit < 8 * 1024 * 1024 {
                "gradle_owasp_report_budget_exceeded"
            } else {
                "gradle_owasp_report_unavailable"
            });
        }
        let Ok(bytes) = read_bounded_regular_file(&path, limit as u64) else {
            fail!("gradle_owasp_report_unavailable");
        };
        let Ok(parsed) = parse_owasp_dependency_check_json(&bytes) else {
            fail!("gradle_owasp_native_report_invalid");
        };
        if parsed.engine_version != "12.1.0" || parsed.project_name != owner.report_project_name {
            fail!("gradle_owasp_native_report_attribution_unresolved");
        }
        if let Err(reason) = budget.reserve_input(bytes.len(), parsed.advisories.len()) {
            fail!(reason);
        }
        let mut row = json!({"project_path":owner.project_path,"task_path":owner.task_path,"implementation":owner.implementation,"report_path":owner.report_path,"report_sha256":digest(&bytes),"engine_version":parsed.engine_version,"project_name":parsed.project_name,"report_date":parsed.report_date,"data_sources":parsed.data_sources,"dependency_count":parsed.dependency_count,"advisories":[]});
        if let Err(reason) = budget.reserve_feedback(&row) {
            fail!(reason);
        }
        let mut advisories = Vec::new();
        for item in parsed.advisories {
            if cancelled.load(Ordering::Relaxed)
                || codeguard_runtime::sigint_cancellation_requested()
            {
                fail!("request_cancelled");
            }
            if Instant::now() >= request.deadline {
                fail!("request_deadline_exceeded");
            }
            let value = json!({"source":item.source,"advisory_id":item.advisory_id,"score":item.score,"package_ids":item.package_ids,"dependency_sha256":item.dependency_sha256,"suppressed_by_native_tool":item.suppressed_by_native_tool});
            if let Err(reason) = budget.reserve_feedback(&value) {
                fail!(reason);
            }
            advisories.push(value);
        }
        row["advisories"] = json!(advisories);
        reports.push(row);
    }
    if cancelled.load(Ordering::Relaxed) || codeguard_runtime::sigint_cancellation_requested() {
        fail!("request_cancelled");
    }
    if Instant::now() >= request.deadline {
        fail!("request_deadline_exceeded");
    }
    report["native_status"] = json!("reports_observed_unverified");
    report["reason"] = json!(if exit == 0 {
        "database_and_dependency_coverage_unverified"
    } else {
        "native_failure_with_reports_unverified"
    });
    report["reports"] = json!(reports);
    report
}
fn native_configuration_blocker(stdout: &[u8], stderr: &[u8]) -> Option<&'static str> {
    let stdout = String::from_utf8_lossy(stdout);
    let stderr = String::from_utf8_lossy(stderr);
    // 只保留固定受管脚本的阻塞码，不将日志自由文本或凭据带入反馈。
    for (marker, reason) in [
        (
            "codeguard_owasp_task_unavailable",
            "gradle_owasp_task_unavailable",
        ),
        (
            "codeguard_owasp_configuration_api_unresolved",
            "gradle_owasp_configuration_api_unresolved",
        ),
        ("codeguard_owasp_scan_skipped", "gradle_owasp_scan_skipped"),
        (
            "codeguard_owasp_json_not_configured",
            "gradle_owasp_json_not_configured",
        ),
        (
            "codeguard_owasp_report_outside_root",
            "gradle_owasp_report_scope_invalid",
        ),
        (
            "codeguard_owasp_report_ownership_duplicate",
            "gradle_owasp_report_ownership_duplicate",
        ),
        (
            "codeguard_owasp_report_preexisting",
            "gradle_owasp_report_preexisting",
        ),
        (
            "codeguard_owasp_composite_build_unresolved",
            "gradle_owasp_composite_build_unresolved",
        ),
        (
            "codeguard_reserved_task_collision",
            "gradle_owasp_reserved_task_collision",
        ),
    ] {
        if stdout.contains(marker) || stderr.contains(marker) {
            return Some(reason);
        }
    }
    None
}
fn valid_task_path(path: &str) -> bool {
    path.len() <= 256
        && path.starts_with(':')
        && path[1..].split(':').all(|part| {
            !part.is_empty()
                && part.len() <= 128
                && part
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-'))
        })
}

fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn private_scratch() -> Result<Scratch, ()> {
    let root = std::env::temp_dir().canonicalize().map_err(|_| ())?;
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| ())?
        .as_nanos();
    let path = root.join(format!(
        "codeguard-gradle-owasp-{}-{nonce}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir(&path).map_err(|_| ())?;
    let scratch = Scratch(path);
    fs::set_permissions(&scratch.0, fs::Permissions::from_mode(0o700)).map_err(|_| ())?;
    Ok(scratch)
}

/// 构造缺前置的观察；不表示原生执行或漏洞清洁。
pub fn missing_prerequisites() -> Value {
    json!({"schema_version":"0.2.0","report_type":"gradle_dependency_check_probe","checker_id":"java.gradle.dependency_check","native_status":"incomplete","reason":"prerequisites_missing","init_scripts_sha256":digest(format!("{MODEL_SCRIPT}\n{OWASP_SCRIPT}").as_bytes()),"dependency_cache_sha256":null,"model_report_sha256":null,"ownership_report_sha256":null,"source_snapshot_sha256":null,"gradle_bundle_sha256":null,"java_entry_sha256":null,"jdk_release_sha256":null,"task_paths":[],"native_exit_code":null,"native_stdout_sha256":null,"native_stderr_sha256":null,"reports":[],"database_freshness_verified":false,"dependency_attribution_verified":false,"coverage_proven":false,"authority":"local_unverified","delivery_decision":"not_evaluated"})
}
