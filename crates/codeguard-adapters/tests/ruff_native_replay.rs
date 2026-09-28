use codeguard_adapters::{RuffParseState, parse_ruff_json};
use std::path::Path;
use std::process::Command;

fn replay(fixture: &str) -> (i32, codeguard_adapters::RuffParsed) {
    let binary = std::env::var_os("CODEGUARD_RUFF_BIN").expect("必须设置 CODEGUARD_RUFF_BIN");
    let version = Command::new(&binary)
        .arg("--version")
        .output()
        .expect("锁定 Ruff 二进制应可启动");
    assert!(version.status.success());
    assert_eq!(
        String::from_utf8_lossy(&version.stdout).trim(),
        "ruff 0.16.8"
    );
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures/corpus")
        .join(fixture);
    let output = Command::new(binary)
        .args(["check", "--isolated", "--output-format", "json"])
        .arg(fixture)
        .output()
        .expect("受控 Ruff 检查应可启动");
    let exit = output.status.code().expect("测试进程应正常退出");
    (exit, parse_ruff_json(exit, &output.stdout))
}

#[test]
#[ignore = "requires pinned Ruff 0.16.8; run with CODEGUARD_RUFF_BIN"]
fn real_ruff_f401_report_matches_native_contract() {
    let (exit, parsed) = replay("ruff_unused_import.py");
    assert_eq!(exit, 1);
    assert_eq!(parsed.state, RuffParseState::Valid);
    assert_eq!(parsed.diagnostics.len(), 1);
    assert_eq!(parsed.diagnostics[0].code, "F401");
}

#[test]
#[ignore = "requires pinned Ruff 0.16.8; run with CODEGUARD_RUFF_BIN"]
fn real_ruff_clean_report_matches_native_contract() {
    let (exit, parsed) = replay("ruff_clean.py");
    assert_eq!(exit, 0);
    assert_eq!(parsed.state, RuffParseState::Valid);
    assert!(parsed.diagnostics.is_empty());
}

#[test]
#[ignore = "requires pinned Ruff 0.16.8; run with CODEGUARD_RUFF_BIN"]
fn real_ruff_configuration_failure_is_incomplete() {
    let binary = std::env::var_os("CODEGUARD_RUFF_BIN").expect("必须设置 CODEGUARD_RUFF_BIN");
    let fixture =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/corpus/ruff_clean.py");
    let output = Command::new(binary)
        .args([
            "check",
            "--isolated",
            "--output-format",
            "json",
            "--select",
            "NOT_A_RULE",
        ])
        .arg(fixture)
        .output()
        .expect("受控 Ruff 错误样本应可启动");
    assert_eq!(output.status.code(), Some(2));
    let parsed = parse_ruff_json(2, &output.stdout);
    assert_eq!(parsed.state, RuffParseState::Incomplete);
    assert!(parsed.diagnostics.is_empty());
}

#[test]
#[ignore = "requires pinned Ruff 0.16.8; run with CODEGUARD_RUFF_BIN"]
fn real_ruff_missing_file_is_environment_failure() {
    let binary = std::env::var_os("CODEGUARD_RUFF_BIN").expect("必须设置 CODEGUARD_RUFF_BIN");
    let missing = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures/corpus/codeguard_nonexistent_ruff_target.py");
    assert!(!missing.exists(), "测试目标必须保持不存在");
    let output = Command::new(binary)
        .args(["check", "--isolated", "--output-format", "json"])
        .arg(missing)
        .output()
        .expect("受控 Ruff 缺文件样本应可启动");
    assert_eq!(output.status.code(), Some(1));
    let parsed = parse_ruff_json(1, &output.stdout);
    assert_eq!(parsed.state, RuffParseState::Incomplete);
    assert!(parsed.diagnostics.is_empty());
    assert_eq!(parsed.environment_diagnostics.len(), 1);
    assert_eq!(parsed.environment_diagnostics[0].code, "E902");
}
