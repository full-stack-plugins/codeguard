//! 调用已有Gradle观察选定构建输入的原生模型；配置执行不等于质量检查。
pub use crate::gradle_model_probe_request::Request;
use crate::tool_identity::hash_bundle_tree;
use codeguard_adapters::parse_gradle_checker_model;
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
const SCRIPT: &str = include_str!("../resources/gradle_checker_model.init.gradle");
static NEXT_SCRATCH: AtomicU64 = AtomicU64::new(0);
struct Scratch(PathBuf);
impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

/// 在私有副本中离线采集模型，返回脱敏局部观察；取消/失败保持未完成，不生成源码违规。
/// 参数含显式工具和选定文件及取消信号；返回值不授予项目覆盖、漏洞清洁或交付结论。
pub fn observe(request: &Request, cancelled: &AtomicBool) -> Value {
    let mut report = json!({"schema_version":"0.1.0","report_type":"gradle_model_probe","native_status":"incomplete","reason":"prerequisites_missing","model":null,"input_scope":"selected_project_files","input_files":[],"source_snapshot_sha256":null,"gradle_bundle_sha256":null,"java_entry_sha256":null,"jdk_release_sha256":null,"init_script_sha256":digest(SCRIPT.as_bytes()),"native_report_sha256":null,"jdk_closure_verified":false,"project_sources_complete":false,"coverage_proven":false,"authority":"local_unverified","delivery_decision":"not_evaluated"});
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
    let has_settings = ["settings.gradle", "settings.gradle.kts"]
        .iter()
        .any(|p| request.project_files.contains(&PathBuf::from(p)));
    let has_build = ["build.gradle", "build.gradle.kts"]
        .iter()
        .any(|p| request.project_files.contains(&PathBuf::from(p)));
    if !has_settings || !has_build {
        fail!("root_build_inputs_missing");
    }
    let Ok(snapshot) = SourceSnapshot::capture(
        &request.project_root,
        request.project_files.iter().cloned(),
        128,
        1024 * 1024,
        16 * 1024 * 1024,
    ) else {
        fail!("project_snapshot_unavailable");
    };
    let files = snapshot
        .files()
        .iter()
        .map(|(path, bytes)| json!({"path":path,"sha256":digest(bytes)}))
        .collect::<Vec<_>>();
    report["source_snapshot_sha256"] = json!(digest(
        &serde_json::to_vec(&files).expect("文件摘要可序列化")
    ));
    report["input_files"] = json!(files);
    let (Ok(bundle), Ok(java_home)) = (
        request.gradle_bundle.canonicalize(),
        request.java_home.canonicalize(),
    ) else {
        fail!("native_prerequisite_unavailable");
    };
    let tool = bundle.join("bin/gradle");
    let java = java_home.join("bin/java");
    let release = java_home.join("release");
    let (Ok(bundle_hash), Ok(java_bytes), Ok(release_bytes)) = (
        hash_bundle_tree(&bundle),
        read_bounded_regular_file(&java, 128 * 1024 * 1024),
        read_bounded_regular_file(&release, 64 * 1024),
    ) else {
        fail!("native_tool_identity_unavailable");
    };
    if release_bytes.is_empty() {
        fail!("jdk_version_unverified");
    }
    report["gradle_bundle_sha256"] = json!(bundle_hash);
    report["java_entry_sha256"] = json!(digest(&java_bytes));
    report["jdk_release_sha256"] = json!(digest(&release_bytes));
    if Instant::now() >= request.deadline {
        fail!("request_deadline_exceeded");
    }
    let Ok(scratch) = private_scratch() else {
        fail!("private_workspace_unavailable");
    };
    let project = scratch.0.join("project");
    let evidence = scratch.0.join("evidence");
    let home = scratch.0.join("home");
    let script = scratch.0.join("model.init.gradle");
    let output = evidence.join("model.json");
    if snapshot.materialize_new(&project).is_err()
        || fs::create_dir(&evidence).is_err()
        || fs::create_dir(&home).is_err()
        || fs::set_permissions(&evidence, fs::Permissions::from_mode(0o700)).is_err()
        || fs::set_permissions(&home, fs::Permissions::from_mode(0o700)).is_err()
        || fs::write(&script, SCRIPT).is_err()
    {
        fail!("private_workspace_unavailable");
    }
    let mut env = BTreeMap::new();
    env.insert(
        OsString::from("JAVA_HOME"),
        java_home.as_os_str().to_owned(),
    );
    env.insert(OsString::from("HOME"), home.as_os_str().to_owned());
    env.insert(OsString::from("PATH"), OsString::from("/usr/bin:/bin"));
    let spec = ProcessSpec {
        executable: tool,
        args: vec![
            "--offline".into(),
            "--no-daemon".into(),
            "--console=plain".into(),
            "--no-configuration-cache".into(),
            "--init-script".into(),
            script.clone().into_os_string(),
            "--project-dir".into(),
            project.clone().into_os_string(),
            "--gradle-user-home".into(),
            home.join(".gradle").into_os_string(),
            format!("-Duser.home={}", home.display()).into(),
            format!("-Dcodeguard.model.output={}", output.display()).into(),
            ":codeguardObserveCheckerModel".into(),
        ],
        cwd: project.clone(),
        env,
        stdin: None,
        deadline: request.deadline,
        output_limit_bytes: 2 * 1024 * 1024,
    };
    let Ok(outcome) = run_process_recorded(&spec, cancelled, &evidence, "gradle.log") else {
        if cancelled.load(Ordering::Relaxed) || codeguard_runtime::sigint_cancellation_requested() {
            fail!("request_cancelled");
        }
        if Instant::now() >= request.deadline {
            fail!("request_deadline_exceeded");
        }
        fail!("native_process_incomplete");
    };
    if cancelled.load(Ordering::Relaxed) || codeguard_runtime::sigint_cancellation_requested() {
        fail!("request_cancelled");
    }
    if Instant::now() >= request.deadline {
        fail!("request_deadline_exceeded");
    }
    if snapshot.verify_unchanged(&project).ok() != Some(true)
        || read_bounded_regular_file(&script, 64 * 1024)
            .ok()
            .as_deref()
            != Some(SCRIPT.as_bytes())
    {
        fail!("native_inputs_changed_during_probe");
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
        fail!("native_identity_changed_during_probe");
    }
    if !matches!(outcome.termination, Termination::Exited(0)) {
        fail!("native_execution_incomplete");
    }
    let Ok(bytes) = read_bounded_regular_file(&output, 1024 * 1024) else {
        fail!("native_model_missing");
    };
    let Ok(model) = parse_gradle_checker_model(&bytes) else {
        fail!("native_model_invalid_or_incomplete");
    };
    if cancelled.load(Ordering::Relaxed) || codeguard_runtime::sigint_cancellation_requested() {
        fail!("request_cancelled");
    }
    if Instant::now() >= request.deadline {
        fail!("request_deadline_exceeded");
    }
    report["native_status"] = json!("model_observed_unverified");
    report["reason"] = json!("selected_inputs_and_jdk_closure_unverified");
    report["model"] = json!(model);
    report["native_report_sha256"] = json!(digest(&bytes));
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
        "codeguard-gradle-model-{}-{nonce}-{}",
        std::process::id(),
        NEXT_SCRATCH.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir(&path).map_err(|_| ())?;
    let scratch = Scratch(path);
    fs::set_permissions(&scratch.0, fs::Permissions::from_mode(0o700)).map_err(|_| ())?;
    Ok(scratch)
}
