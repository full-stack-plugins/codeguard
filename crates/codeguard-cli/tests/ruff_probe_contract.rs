#![cfg(unix)]

use codeguard_cli::ruff_probe::{
    RuffConfigBinding, RuffProbeRequest, RuffProbeState, run_ruff_probe,
};
use sha2::{Digest, Sha256};
use std::fs;
use std::os::unix::fs::{PermissionsExt, symlink};
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;
use std::time::{Duration, Instant};

fn digest(path: &Path) -> [u8; 32] {
    Sha256::digest(fs::read(path).expect("读取工具")).into()
}

fn request(source: PathBuf, tool: PathBuf, evidence_dir: PathBuf) -> RuffProbeRequest {
    RuffProbeRequest {
        source,
        tool: tool.clone(),
        expected_tool_sha256: digest(&tool),
        expected_version: "ruff 0.16.8".into(),
        evidence_dir,
        run_id: format!("probe-{}", std::process::id()),
        deadline: Instant::now() + Duration::from_secs(5),
        project_config: None,
    }
}

#[test]
fn rejects_a_missing_source_before_native_execution() {
    let source = std::env::temp_dir().join("codeguard-ruff-probe-never-created.py");
    assert!(!source.exists());
    let result = run_ruff_probe(
        &RuffProbeRequest {
            source,
            tool: "/nonexistent/ruff".into(),
            expected_tool_sha256: [0; 32],
            expected_version: "ruff 0.16.8".into(),
            evidence_dir: std::env::temp_dir(),
            run_id: "missing-source".into(),
            deadline: Instant::now() + Duration::from_secs(2),
            project_config: None,
        },
        &AtomicBool::new(false),
    );
    assert_eq!(result.state, RuffProbeState::Incomplete);
    assert_eq!(result.reason, Some("source_unavailable"));
    assert!(result.parsed.is_none());
}

#[test]
fn rejects_mismatched_tool_content_before_version_or_scan() {
    let source =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/corpus/ruff_clean.py");
    let result = run_ruff_probe(
        &RuffProbeRequest {
            source,
            tool: "/bin/echo".into(),
            expected_tool_sha256: [0; 32],
            expected_version: "ruff 0.16.8".into(),
            evidence_dir: std::env::temp_dir(),
            run_id: "wrong-tool".into(),
            deadline: Instant::now() + Duration::from_secs(2),
            project_config: None,
        },
        &AtomicBool::new(false),
    );
    assert_eq!(result.state, RuffProbeState::Incomplete);
    assert_eq!(result.reason, Some("tool_identity_mismatch"));
    assert!(result.parsed.is_none());
}

#[test]
fn native_report_for_another_file_cannot_be_attributed_to_the_requested_source() {
    let root = std::env::temp_dir()
        .canonicalize()
        .unwrap()
        .join(format!("codeguard-ruff-wrong-file-{}", std::process::id()));
    fs::create_dir(&root).expect("fixture root");
    let source = root.join("app.py");
    fs::write(&source, "import os\n").expect("source");
    let other = root.join("other.py");
    fs::write(&other, "pass\n").expect("other source");
    let tool = root.join("fake-ruff");
    let report = format!(
        r#"[{{"code":"F401","message":"unused import","filename":"{}","location":{{"row":1,"column":1}},"severity":"error"}},{{"code":"E501","message":"wrong file","filename":"{}","location":{{"row":1,"column":1}},"severity":"error"}}]"#,
        source.display(),
        other.display()
    );
    fs::write(
        &tool,
        format!(
            "#!/bin/sh\nif [ \"$1\" = \"--version\" ]; then printf 'ruff 0.16.8\\n'; else printf '%s\\n' '{report}'; exit 1; fi\n"
        ),
    )
    .expect("fake native checker");
    fs::set_permissions(&tool, fs::Permissions::from_mode(0o700)).expect("executable");
    let evidence = root.join("evidence");
    fs::create_dir(&evidence).expect("evidence directory");
    fs::set_permissions(&evidence, fs::Permissions::from_mode(0o700)).expect("private evidence");
    let result = run_ruff_probe(&request(source, tool, evidence), &AtomicBool::new(false));
    assert_eq!(result.state, RuffProbeState::Incomplete);
    assert_eq!(result.reason, Some("report_source_mismatch"));
    let diagnostics = result.parsed.expect("retain same-file finding").diagnostics;
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].code, "F401");
    fs::remove_dir_all(root).expect("remove fixture");
}

#[test]
#[ignore = "requires pinned Ruff 0.16.8; run with CODEGUARD_RUFF_BIN"]
fn real_native_ruff_is_invoked_through_the_runtime_with_private_evidence() {
    let tool = PathBuf::from(std::env::var_os("CODEGUARD_RUFF_BIN").expect("Ruff 路径"));
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures/corpus/ruff_unused_import.py");
    let evidence_dir = std::env::temp_dir()
        .canonicalize()
        .expect("规范化临时目录")
        .join(format!("codeguard-ruff-probe-{}", std::process::id()));
    fs::create_dir(&evidence_dir).expect("证据目录");
    fs::set_permissions(&evidence_dir, fs::Permissions::from_mode(0o700)).expect("私有目录");
    let result = run_ruff_probe(
        &request(fixture, tool, evidence_dir.clone()),
        &AtomicBool::new(false),
    );
    assert_eq!(result.state, RuffProbeState::EvidenceComplete);
    assert_eq!(
        result.parsed.as_ref().expect("报告").diagnostics[0].code,
        "F401"
    );
    assert!(
        evidence_dir
            .join(format!("probe-{}-version.log", std::process::id()))
            .exists()
    );
    assert!(
        evidence_dir
            .join(format!("probe-{}-scan.log", std::process::id()))
            .exists()
    );
    fs::remove_dir_all(evidence_dir).expect("清理测试证据");
}

#[test]
#[ignore = "requires pinned Ruff 0.16.8; run with CODEGUARD_RUFF_BIN"]
fn clean_native_ruff_report_is_complete_local_evidence() {
    let tool = PathBuf::from(std::env::var_os("CODEGUARD_RUFF_BIN").expect("Ruff 路径"));
    let fixture =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/corpus/ruff_clean.py");
    let evidence_dir = std::env::temp_dir()
        .canonicalize()
        .expect("规范化临时目录")
        .join(format!("codeguard-ruff-clean-{}", std::process::id()));
    fs::create_dir(&evidence_dir).expect("证据目录");
    fs::set_permissions(&evidence_dir, fs::Permissions::from_mode(0o700)).expect("私有目录");
    let result = run_ruff_probe(
        &request(fixture, tool, evidence_dir.clone()),
        &AtomicBool::new(false),
    );
    assert_eq!(result.state, RuffProbeState::EvidenceComplete);
    assert!(result.parsed.as_ref().expect("报告").diagnostics.is_empty());
    fs::remove_dir_all(evidence_dir).expect("清理测试证据");
}

#[test]
#[ignore = "requires pinned Ruff 0.16.8; run with CODEGUARD_RUFF_BIN"]
fn symlinked_evidence_path_cannot_turn_a_native_finding_into_success() {
    let tool = PathBuf::from(std::env::var_os("CODEGUARD_RUFF_BIN").expect("Ruff 路径"));
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures/corpus/ruff_unused_import.py");
    let evidence_dir = std::env::temp_dir()
        .canonicalize()
        .expect("规范化临时目录")
        .join(format!("codeguard-ruff-symlink-{}", std::process::id()));
    fs::create_dir(&evidence_dir).expect("证据目录");
    fs::set_permissions(&evidence_dir, fs::Permissions::from_mode(0o700)).expect("私有目录");
    let target = evidence_dir.join("external-target");
    fs::write(&target, b"untouched").expect("目标");
    symlink(
        &target,
        evidence_dir.join(format!("probe-{}-scan.log", std::process::id())),
    )
    .expect("符号链接");
    let result = run_ruff_probe(
        &request(fixture, tool, evidence_dir.clone()),
        &AtomicBool::new(false),
    );
    assert_eq!(result.state, RuffProbeState::Incomplete);
    assert_eq!(result.reason, Some("evidence_write_failed"));
    assert_eq!(fs::read(target).expect("目标仍存在"), b"untouched");
    fs::remove_dir_all(evidence_dir).expect("清理测试证据");
}

#[test]
#[ignore = "requires pinned Ruff 0.16.8; run with CODEGUARD_RUFF_BIN"]
fn configured_project_rule_is_applied_without_isolated_mode() {
    let tool = PathBuf::from(std::env::var_os("CODEGUARD_RUFF_BIN").expect("Ruff 路径"));
    let root = std::env::temp_dir()
        .canonicalize()
        .unwrap()
        .join(format!("codeguard-ruff-configured-{}", std::process::id()));
    fs::create_dir(&root).expect("项目目录");
    let source = root.join("app.py");
    let config = root.join("pyproject.toml");
    fs::write(&source, format!("x = \"{}\"\n", "a".repeat(120))).expect("源码");
    fs::write(&config, "[tool.ruff.lint]\nselect = ['E501']\n").expect("配置");
    let evidence = root.join("evidence");
    fs::create_dir(&evidence).expect("证据目录");
    fs::set_permissions(&evidence, fs::Permissions::from_mode(0o700)).expect("私有目录");
    let mut probe = request(source, tool, evidence);
    probe.project_config = Some(RuffConfigBinding {
        path: config.clone(),
        expected_sha256: digest(&config),
    });
    let result = run_ruff_probe(&probe, &AtomicBool::new(false));
    assert_eq!(
        result.state,
        RuffProbeState::EvidenceComplete,
        "{:?}",
        result.reason
    );
    let diagnostics = &result.parsed.as_ref().expect("原生报告").diagnostics;
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].code, "E501");
    fs::remove_dir_all(root).expect("清理测试目录");
}

#[test]
#[ignore = "requires pinned Ruff 0.16.8; run with CODEGUARD_RUFF_BIN"]
fn excluded_source_cannot_produce_a_false_clean_configured_probe() {
    let tool = PathBuf::from(std::env::var_os("CODEGUARD_RUFF_BIN").expect("Ruff 路径"));
    let root = std::env::temp_dir()
        .canonicalize()
        .unwrap()
        .join(format!("codeguard-ruff-excluded-{}", std::process::id()));
    fs::create_dir(&root).expect("项目目录");
    let source = root.join("app.py");
    let config = root.join("ruff.toml");
    fs::write(&source, "import os\n").expect("源码");
    fs::write(
        &config,
        "force-exclude = true\nexclude = ['app.py']\n[lint]\nselect = ['F']\n",
    )
    .expect("配置");
    let evidence = root.join("evidence");
    fs::create_dir(&evidence).expect("证据目录");
    fs::set_permissions(&evidence, fs::Permissions::from_mode(0o700)).expect("私有目录");
    let mut probe = request(source, tool, evidence);
    probe.project_config = Some(RuffConfigBinding {
        path: config.clone(),
        expected_sha256: digest(&config),
    });
    let result = run_ruff_probe(&probe, &AtomicBool::new(false));
    assert_eq!(result.state, RuffProbeState::Incomplete);
    assert_eq!(result.reason, Some("source_not_selected_by_ruff"));
    fs::remove_dir_all(root).expect("清理测试目录");
}

#[test]
fn configured_probe_rejects_format_only_config_before_execution() {
    let root = std::env::temp_dir()
        .canonicalize()
        .unwrap()
        .join(format!("codeguard-ruff-format-only-{}", std::process::id()));
    fs::create_dir(&root).expect("项目目录");
    let source = root.join("app.py");
    let config = root.join("ruff.toml");
    fs::write(&source, "pass\n").expect("源码");
    fs::write(&config, "[format]\nquote-style = 'single'\n").expect("配置");
    let mut probe = request(source, "/bin/echo".into(), root.clone());
    probe.project_config = Some(RuffConfigBinding {
        path: config.clone(),
        expected_sha256: digest(&config),
    });
    let result = run_ruff_probe(&probe, &AtomicBool::new(false));
    assert_eq!(result.state, RuffProbeState::Incomplete);
    assert_eq!(result.reason, Some("ruff_config_not_validated"));
    fs::remove_dir_all(root).expect("清理测试目录");
}

#[test]
fn configured_probe_rejects_a_higher_precedence_config() {
    let root = std::env::temp_dir()
        .canonicalize()
        .unwrap()
        .join(format!("codeguard-ruff-precedence-{}", std::process::id()));
    fs::create_dir(&root).expect("项目目录");
    let source = root.join("app.py");
    let config = root.join("pyproject.toml");
    fs::write(&source, "pass\n").expect("源码");
    fs::write(&config, "[tool.ruff.lint]\nselect = ['F']\n").expect("配置");
    fs::write(root.join(".ruff.toml"), "[lint]\nselect = ['E']\n").expect("更高优先级配置");
    let mut probe = request(source, "/bin/echo".into(), root.clone());
    probe.project_config = Some(RuffConfigBinding {
        path: config.clone(),
        expected_sha256: digest(&config),
    });
    let result = run_ruff_probe(&probe, &AtomicBool::new(false));
    assert_eq!(result.state, RuffProbeState::Incomplete);
    assert_eq!(result.reason, Some("ruff_config_selection_changed"));
    fs::remove_dir_all(root).expect("清理测试目录");
}
