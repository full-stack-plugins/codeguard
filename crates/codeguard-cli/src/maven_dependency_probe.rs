//! Maven Dependency Plugin 私有原 POM 探针；仅对静态简单项目读取原生依赖图。

use std::collections::BTreeMap;
use std::ffi::OsString;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use codeguard_adapters::{
    MavenDependencyNode, dependency_pom_project_identity, parse_maven_dependency_tree_json,
};
use codeguard_runtime::{
    ProcessSpec, SourceSnapshot, Termination, read_bounded_regular_file, run_process_recorded,
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

use crate::tool_identity::hash_bundle_tree;

const SETTINGS: &str = include_str!("../resources/isolated_maven_settings.xml");

/// 单构建根依赖图探针所需的原生工具和输入身份。
pub(crate) struct Request<'a> {
    pub build_root: &'a Path,
    pub expected_pom_sha256: &'a str,
    pub maven_tool: Option<&'a Path>,
    pub java_home: Option<&'a Path>,
    pub maven_repo: Option<&'a Path>,
    pub repo_sha256: Option<&'a str>,
    pub deadline: Instant,
    pub cancelled: &'a AtomicBool,
}

struct Scratch(PathBuf);

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

/// 在私有原 POM 快照中执行原生 dependency:tree；返回局部未授权观察。
pub(crate) fn observe(request: &Request<'_>) -> Value {
    let mut report = incomplete("prerequisites_missing");
    let (Some(maven), Some(java_home), Some(repo), Some(repo_digest)) = (
        request.maven_tool,
        request.java_home,
        request.maven_repo,
        request.repo_sha256,
    ) else {
        return report;
    };
    if request.cancelled.load(Ordering::Relaxed)
        || codeguard_runtime::sigint_cancellation_requested()
    {
        return reason(report, "request_cancelled");
    }
    if Instant::now() >= request.deadline {
        return reason(report, "request_deadline_exceeded");
    }
    let Ok(snapshot) = SourceSnapshot::capture(
        request.build_root,
        [PathBuf::from("pom.xml")],
        1,
        4 * 1024 * 1024,
        4 * 1024 * 1024,
    ) else {
        return reason(report, "project_pom_unavailable");
    };
    let pom = snapshot
        .files()
        .get(Path::new("pom.xml"))
        .expect("已捕获 POM");
    if digest(pom) != request.expected_pom_sha256 {
        return reason(report, "project_pom_changed_before_scan");
    }
    let Some((group, artifact, version)) = dependency_pom_project_identity(pom) else {
        return reason(report, "project_pom_replay_ineligible");
    };
    report["native_plan_sha256"] = json!(digest(pom));
    let (Ok(maven), Ok(java_home), Ok(repo)) = (
        maven.canonicalize(),
        java_home.canonicalize(),
        repo.canonicalize(),
    ) else {
        return reason(report, "native_prerequisite_unavailable");
    };
    let java = java_home.join("bin/java");
    let (Ok(maven_bytes), Ok(java_bytes), Ok(release_bytes)) = (
        read_bounded_regular_file(&maven, 128 * 1024 * 1024),
        read_bounded_regular_file(&java, 128 * 1024 * 1024),
        read_bounded_regular_file(&java_home.join("release"), 64 * 1024),
    ) else {
        return reason(report, "native_tool_identity_unavailable");
    };
    if release_bytes.is_empty() {
        return reason(report, "jdk_version_unverified");
    }
    if repo_digest.len() != 64
        || !repo_digest.bytes().all(|byte| byte.is_ascii_hexdigit())
        || hash_bundle_tree(&repo).ok().as_deref() != Some(repo_digest)
    {
        return reason(report, "dependency_closure_mismatch");
    }
    report["maven_tool_sha256"] = json!(digest(&maven_bytes));
    report["java_runtime_sha256"] = json!(digest(&java_bytes));
    report["dependency_closure_sha256"] = json!(repo_digest);
    let Ok(scratch) = private_scratch() else {
        return reason(report, "private_workspace_unavailable");
    };
    let project = scratch.0.join("project");
    let evidence = scratch.0.join("evidence");
    if snapshot.materialize_new(&project).is_err()
        || fs::write(project.join("settings.xml"), SETTINGS).is_err()
        || fs::create_dir(&evidence).is_err()
        || fs::set_permissions(&evidence, fs::Permissions::from_mode(0o700)).is_err()
    {
        return reason(report, "private_workspace_unavailable");
    }
    let mut env = BTreeMap::new();
    env.insert(
        OsString::from("JAVA_HOME"),
        java_home.as_os_str().to_owned(),
    );
    env.insert(OsString::from("MAVEN_SKIP_RC"), OsString::from("1"));
    env.insert(
        OsString::from("PATH"),
        OsString::from(format!("{}:/usr/bin:/bin", java_home.join("bin").display())),
    );
    let spec = ProcessSpec {
        executable: maven.clone(),
        args: vec![
            "-B".into(),
            "-ntp".into(),
            "-o".into(),
            "-s".into(),
            project.join("settings.xml").into_os_string(),
            "-gs".into(),
            project.join("settings.xml").into_os_string(),
            format!("-Dmaven.repo.local={}", repo.display()).into(),
            "org.apache.maven.plugins:maven-dependency-plugin:3.8.1:tree".into(),
            "-DoutputType=json".into(),
            format!("-DoutputFile={}", evidence.join("tree.json").display()).into(),
        ],
        cwd: project.clone(),
        env,
        stdin: None,
        deadline: request.deadline,
        output_limit_bytes: 2 * 1024 * 1024,
    };
    let outcome = match run_process_recorded(&spec, request.cancelled, &evidence, "dependency.log")
    {
        Ok(value) => value,
        Err(_) => return reason(report, "native_process_incomplete"),
    };
    if snapshot.verify_unchanged(&project).ok() != Some(true)
        || read_bounded_regular_file(&project.join("settings.xml"), 64 * 1024)
            .ok()
            .as_deref()
            != Some(SETTINGS.as_bytes())
    {
        return reason(report, "native_inputs_changed_during_scan");
    }
    if read_bounded_regular_file(&maven, 128 * 1024 * 1024)
        .ok()
        .as_deref()
        != Some(maven_bytes.as_slice())
        || read_bounded_regular_file(&java, 128 * 1024 * 1024)
            .ok()
            .as_deref()
            != Some(java_bytes.as_slice())
        || hash_bundle_tree(&repo).ok().as_deref() != Some(repo_digest)
    {
        return reason(report, "native_identity_changed_during_scan");
    }
    if !matches!(outcome.termination, Termination::Exited(0)) || !outcome.stderr.is_empty() {
        return reason(report, "native_execution_incomplete");
    }
    if let Err(why) = validate_native_log(&outcome.stdout) {
        return reason(report, why);
    }
    let Ok(bytes) = read_bounded_regular_file(&evidence.join("tree.json"), 4 * 1024 * 1024) else {
        return reason(report, "native_output_missing");
    };
    let Ok(tree) = parse_maven_dependency_tree_json(&bytes) else {
        return reason(report, "native_graph_invalid");
    };
    if tree.nodes[0].group_id != group
        || tree.nodes[0].artifact_id != artifact
        || tree.nodes[0].version != version
    {
        return reason(report, "native_graph_project_mismatch");
    }
    report["native_status"] = json!("graph_observed_untrusted");
    report["reason"] = json!("effective_model_and_full_scope_unverified");
    report["graph_sha256"] = json!(digest(&bytes));
    let artifact_digests: Vec<_> = tree
        .nodes
        .iter()
        .enumerate()
        .map(|(index, node)| {
            if index == 0 {
                None
            } else {
                artifact_digest(&repo, node)
            }
        })
        .collect();
    if hash_bundle_tree(&repo).ok().as_deref() != Some(repo_digest) {
        return reason(report, "native_identity_changed_during_scan");
    }
    report["nodes"] = json!(
        tree.nodes
            .iter()
            .enumerate()
            .map(|(index, node)| json!({
                "group_id":node.group_id,"artifact_id":node.artifact_id,
                "version":node.version,"type":node.artifact_type,"scope":node.scope,
                "classifier":node.classifier,"optional":node.optional,
                "artifact_sha256":artifact_digests[index]
            }))
            .collect::<Vec<_>>()
    );
    report["edges"] = json!(tree.edges);
    report
}

fn artifact_digest(repo: &Path, node: &MavenDependencyNode) -> Option<String> {
    let extension = match node.artifact_type.as_str() {
        "jar" | "pom" | "war" | "zip" | "aar" => node.artifact_type.as_str(),
        _ => return None,
    };
    let group: Vec<_> = node.group_id.split('.').collect();
    let components = group
        .iter()
        .copied()
        .chain([node.artifact_id.as_str(), node.version.as_str()]);
    if components
        .clone()
        .any(|part| part.is_empty() || matches!(part, "." | ".."))
        || (!node.classifier.is_empty() && matches!(node.classifier.as_str(), "." | ".."))
    {
        return None;
    }
    let mut path = repo.to_path_buf();
    for part in components {
        path.push(part);
    }
    let classifier = if node.classifier.is_empty() {
        String::new()
    } else {
        format!("-{}", node.classifier)
    };
    path.push(format!(
        "{}-{}{}.{}",
        node.artifact_id, node.version, classifier, extension
    ));
    let bytes = read_bounded_regular_file(&path, 128 * 1024 * 1024).ok()?;
    Some(digest(&bytes))
}

/// 同形状未完成结果，防止执行前错误被解释为空图。
pub(crate) fn incomplete(why: &'static str) -> Value {
    json!({
        "schema_version":"0.2.0","report_type":"maven_dependency_tree_probe",
        "checker_id":"java.maven.dependency","category":"dependencies",
        "native_status":"incomplete","reason":why,
        "native_plan_sha256":null,"maven_tool_sha256":null,
        "java_runtime_sha256":null,"dependency_closure_sha256":null,
        "graph_sha256":null,"nodes":[],"edges":[],
        "coverage_proven":false,"authority":"local_unverified",
        "delivery_decision":"not_evaluated"
    })
}

fn reason(mut report: Value, why: &'static str) -> Value {
    report["reason"] = json!(why);
    report
}

fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn validate_native_log(bytes: &[u8]) -> Result<(), &'static str> {
    if bytes.is_empty() || bytes.len() > 2 * 1024 * 1024 {
        return Err("native_log_invalid");
    }
    let log = std::str::from_utf8(bytes).map_err(|_| "native_log_invalid")?;
    if log
        .bytes()
        .any(|byte| byte.is_ascii_control() && !matches!(byte, b'\n' | b'\r' | b'\t'))
    {
        return Err("native_log_invalid");
    }
    let lines: Vec<_> = log
        .lines()
        .map(|line| line.trim_end_matches('\r'))
        .collect();
    if lines.iter().any(|line| {
        line.starts_with("[WARNING]")
            || line.starts_with("[ERROR]")
            || line.contains("unavailable in current build context")
    }) {
        return Err("native_dependency_resolution_warning");
    }
    if lines
        .iter()
        .filter(|line| line.starts_with("[INFO] --- dependency:3.8.1:tree "))
        .count()
        != 1
        || lines
            .iter()
            .filter(|line| **line == "[INFO] BUILD SUCCESS")
            .count()
            != 1
        || lines.iter().any(|line| line == &"[INFO] BUILD FAILURE")
    {
        return Err("native_dependency_goal_unverified");
    }
    Ok(())
}

fn private_scratch() -> Result<Scratch, &'static str> {
    let root = std::env::temp_dir()
        .canonicalize()
        .map_err(|_| "temp_root_unavailable")?;
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| "clock_unavailable")?
        .as_nanos();
    let path = root.join(format!(
        "codeguard-maven-dependency-{}-{nonce}",
        std::process::id()
    ));
    fs::create_dir(&path).map_err(|_| "scratch_create_failed")?;
    fs::set_permissions(&path, fs::Permissions::from_mode(0o700))
        .map_err(|_| "scratch_permission_failed")?;
    Ok(Scratch(path))
}

#[cfg(test)]
mod tests {
    use super::{Request, observe, private_scratch, validate_native_log};
    use sha2::{Digest, Sha256};
    use std::fs;
    use std::path::Path;
    use std::sync::atomic::AtomicBool;
    use std::time::{Duration, Instant};

    #[test]
    fn inherited_pom_is_rejected_before_native_tool_resolution() {
        let scratch = private_scratch().unwrap();
        let root = scratch.0.join("project");
        fs::create_dir(&root).unwrap();
        let pom = b"<project><modelVersion>4.0.0</modelVersion><parent><groupId>x</groupId><artifactId>p</artifactId><version>1</version></parent><groupId>demo</groupId><artifactId>app</artifactId><version>1</version><build><plugins><plugin><artifactId>maven-dependency-plugin</artifactId><version>3.8.1</version></plugin></plugins></build></project>";
        fs::write(root.join("pom.xml"), pom).unwrap();
        let digest = format!("{:x}", Sha256::digest(pom));
        let cancelled = AtomicBool::new(false);
        let report = observe(&Request {
            build_root: &root,
            expected_pom_sha256: &digest,
            maven_tool: Some(Path::new("/nonexistent/maven")),
            java_home: Some(Path::new("/nonexistent/jdk")),
            maven_repo: Some(Path::new("/nonexistent/repo")),
            repo_sha256: Some("0"),
            deadline: Instant::now() + Duration::from_secs(5),
            cancelled: &cancelled,
        });
        assert_eq!(report["native_status"], "incomplete");
        assert_eq!(report["reason"], "project_pom_replay_ineligible");
        assert!(report["nodes"].as_array().unwrap().is_empty());
    }

    #[test]
    fn build_success_with_missing_dependency_pom_is_not_a_complete_graph() {
        let clean =
            b"[INFO] --- dependency:3.8.1:tree (default-cli) @ demo ---\n[INFO] BUILD SUCCESS\n";
        assert_eq!(validate_native_log(clean), Ok(()));
        let missing = b"[INFO] Artifact junit:junit:pom:4.13.2 is present in the local repository, but cached from a remote repository ID that is unavailable in current build context\n[WARNING] The POM for junit:junit:jar:4.13.2 is missing, no dependency information available\n[INFO] --- dependency:3.8.1:tree (default-cli) @ demo ---\n[INFO] BUILD SUCCESS\n";
        assert_eq!(
            validate_native_log(missing),
            Err("native_dependency_resolution_warning")
        );
        assert_eq!(
            validate_native_log(b"[INFO] BUILD SUCCESS\n"),
            Err("native_dependency_goal_unverified")
        );
    }
}
