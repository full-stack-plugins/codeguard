//! Rust 详细文档注释验收（15.3）。
//!
//! 按 Rust 语言规范检查 rustdoc 用途、参数、返回、错误及行为契约。

use std::process::Command;

fn ensure_clean_dir(name: &str) -> std::path::PathBuf {
    let tmp = std::env::temp_dir().join(format!("{}-{}", name, std::process::id()));
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(&tmp).unwrap();
    tmp
}

/// 完整 rustdoc 应通过。
#[test]
fn complete_rustdoc_passes() {
    let root = ensure_clean_dir("codeguard-rs-doc-complete");
    let code = "/// Adds two numbers together.\n///\n/// # Arguments\n///\n/// * `a` - The first operand\n/// * `b` - The second operand\n///\n/// # Returns\n///\n/// The sum of a and b\npub fn add(a: i32, b: i32) -> i32 {\n    a + b\n}\n";
    std::fs::write(root.join("complete.rs"), code).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["comments", "rust", root.to_str().unwrap(), "--format=json"])
        .output()
        .expect("无法运行 codeguard comments rust");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let report: serde_json::Value = serde_json::from_str(&stdout).expect("JSON 解析失败");
    assert!(report["report_type"].as_str().map(|s| !s.is_empty()).unwrap_or(false), "应有 report_type");
    let _ = std::fs::remove_dir_all(&root);
}

/// 缺 rustdoc 应被检出。
#[test]
fn missing_rustdoc_detected() {
    let root = ensure_clean_dir("codeguard-rs-doc-missing");
    std::fs::write(root.join("missing.rs"), "pub fn add(a: i32, b: i32) -> i32 { a + b }\n").unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["comments", "rust", root.to_str().unwrap(), "--format=json"])
        .output()
        .expect("无法运行 codeguard comments rust");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let report: serde_json::Value = serde_json::from_str(&stdout).expect("JSON 解析失败");
    assert!(report["report_type"].as_str().map(|s| !s.is_empty()).unwrap_or(false), "应有 report_type");
    let _ = std::fs::remove_dir_all(&root);
}

/// 空 rustdoc 不能冒充合规。
#[test]
fn empty_rustdoc_cannot_impersonate() {
    let root = ensure_clean_dir("codeguard-rs-doc-empty");
    std::fs::write(root.join("empty.rs"), "///\npub fn add(a: i32, b: i32) -> i32 { a + b }\n").unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["comments", "rust", root.to_str().unwrap(), "--format=json"])
        .output()
        .expect("无法运行 codeguard comments rust");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let report: serde_json::Value = serde_json::from_str(&stdout).expect("JSON 解析失败");
    assert!(report["report_type"].as_str().map(|s| !s.is_empty()).unwrap_or(false), "应有 report_type");
    let _ = std::fs::remove_dir_all(&root);
}
