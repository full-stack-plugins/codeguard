//! 执行原项目已观察到的Gradle Javadoc任务；不替换项目规则或签发注释合规。
use crate::{gradle_model_probe::Request, tool_identity::hash_bundle_tree};
use codeguard_adapters::{
    parse_gradle_checker_model, parse_gradle_javadoc_output, plan_gradle_javadoc_tasks,
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
const JAVADOC_SCRIPT: &str = include_str!("../resources/gradle_javadoc.init.gradle");
/// 在同一私有输入副本的一次Gradle调用中记录模型并强制重跑启用的官方任务。
/// 参数为显式工具/构建文件/Java文件/截止时间及取消信号；返回局部诊断而非完整注释合规。
pub fn observe(request: &Request, cancelled: &AtomicBool) -> Value {
    let mut report = missing_prerequisites();
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
    let Ok(scratch) = private_scratch() else {
        fail!("private_workspace_unavailable");
    };
    let project = scratch.0.join("project");
    let evidence = scratch.0.join("evidence");
    let home = scratch.0.join("home");
    let model_script = scratch.0.join("model.init.gradle");
    let javadoc_script = scratch.0.join("javadoc.init.gradle");
    let model_output = evidence.join("model.json");
    if snapshot.materialize_new(&project).is_err()
        || fs::create_dir(&evidence).is_err()
        || fs::create_dir(&home).is_err()
        || fs::write(&model_script, MODEL_SCRIPT).is_err()
        || fs::write(&javadoc_script, JAVADOC_SCRIPT).is_err()
        || fs::set_permissions(&evidence, fs::Permissions::from_mode(0o700)).is_err()
        || fs::set_permissions(&home, fs::Permissions::from_mode(0o700)).is_err()
    {
        fail!("private_workspace_unavailable");
    }
    let sources = snapshot
        .files()
        .iter()
        .filter(|(path, _)| path.extension().is_some_and(|s| s == "java"))
        .map(|(path, bytes)| {
            (
                project.join(path).to_string_lossy().into_owned(),
                bytes.clone(),
            )
        })
        .collect::<BTreeMap<_, _>>();
    report["selected_java_file_count"] = json!(sources.len());
    if sources.is_empty() {
        fail!("selected_java_sources_missing");
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
        javadoc_script.clone().into_os_string(),
        format!("-Dcodeguard.model.output={}", model_output.display()).into(),
        "--project-dir".into(),
        project.clone().into_os_string(),
        "--gradle-user-home".into(),
        home.join(".gradle").into_os_string(),
        format!("-Duser.home={}", home.display()).into(),
        "-Duser.language=en".into(),
        "-Duser.country=US".into(),
        "-Dorg.gradle.parallel=false".into(),
    ];
    args.push(":codeguardRunJavadoc".into());
    let spec = ProcessSpec {
        executable: bundle.join("bin/gradle"),
        args,
        cwd: project.clone(),
        env,
        stdin: None,
        deadline: request.deadline,
        output_limit_bytes: 2 * 1024 * 1024,
    };
    let outcome = run_process_recorded(&spec, cancelled, &evidence, "gradle-javadoc.log");
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
        || read_bounded_regular_file(&javadoc_script, 64 * 1024)
            .ok()
            .as_deref()
            != Some(JAVADOC_SCRIPT.as_bytes())
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
    if !matches!(outcome.termination, Termination::Exited(0)) {
        fail!("native_execution_incomplete");
    }
    let Ok(model_bytes) = read_bounded_regular_file(&model_output, 1024 * 1024) else {
        fail!("gradle_native_model_incomplete");
    };
    let Ok(model) = parse_gradle_checker_model(&model_bytes) else {
        fail!("gradle_native_model_invalid");
    };
    let Ok(plans) = plan_gradle_javadoc_tasks(&model) else {
        fail!("gradle_javadoc_task_scope_invalid");
    };
    if plans.is_empty() {
        fail!("gradle_javadoc_task_unavailable");
    }
    report["model_report_sha256"] = json!(digest(&model_bytes));
    report["task_paths"] = json!(plans.into_iter().map(|p| p.task_path).collect::<Vec<_>>());
    report["native_stdout_sha256"] = json!(digest(&outcome.stdout));
    report["native_stderr_sha256"] = json!(digest(&outcome.stderr));
    if String::from_utf8_lossy(&outcome.stdout).contains("warning:")
        || String::from_utf8_lossy(&outcome.stdout).contains("error:")
    {
        fail!("native_stdout_diagnostics_unresolved");
    }
    let Ok(diagnostics) = parse_gradle_javadoc_output(&outcome.stderr, &sources) else {
        fail!("native_javadoc_output_unresolved");
    };
    let mut findings = Vec::new();
    for d in diagnostics {
        let Ok(relative) = std::path::Path::new(&d.path).strip_prefix(&project) else {
            fail!("native_javadoc_source_unresolved");
        };
        findings.push(json!({"path":relative,"rule_id":d.rule_id,"line":d.line,"column":d.column}));
    }
    if cancelled.load(Ordering::Relaxed) || codeguard_runtime::sigint_cancellation_requested() {
        fail!("request_cancelled");
    }
    if Instant::now() >= request.deadline {
        fail!("request_deadline_exceeded");
    }
    report["native_status"] = json!(if findings.is_empty() {
        "empty_output_unverified"
    } else {
        "findings_observed_unverified"
    });
    report["reason"] = json!("selected_sources_and_complete_documentation_rules_unverified");
    report["findings"] = json!(findings);
    report
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
        "codeguard-gradle-javadoc-{}-{nonce}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir(&path).map_err(|_| ())?;
    let scratch = Scratch(path);
    fs::set_permissions(&scratch.0, fs::Permissions::from_mode(0o700)).map_err(|_| ())?;
    Ok(scratch)
}

/// 构造未提供原工具的准备失败观察，供绑定输入的修复任务使用；不宣称原生执行。
pub(crate) fn missing_prerequisites() -> Value {
    json!({"schema_version":"0.2.0","report_type":"gradle_javadoc_probe","native_status":"incomplete","reason":"prerequisites_missing","init_scripts_sha256":digest(format!("{MODEL_SCRIPT}\n{JAVADOC_SCRIPT}").as_bytes()),"model_report_sha256":null,"source_snapshot_sha256":null,"gradle_bundle_sha256":null,"java_entry_sha256":null,"jdk_release_sha256":null,"task_paths":[],"selected_java_file_count":0,"native_stdout_sha256":null,"native_stderr_sha256":null,"findings":[],"rule_configuration_complete":false,"coverage_proven":false,"authority":"local_unverified","delivery_decision":"not_evaluated"})
}
