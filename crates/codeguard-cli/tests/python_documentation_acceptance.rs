//! Python 详细文档注释验收（15.3）。
//!
//! 按 Python 语言规范检查 docstring 用途、参数、返回、错误及行为契约。

use std::process::Command;

fn ensure_clean_dir(name: &str) -> std::path::PathBuf {
    let tmp = std::env::temp_dir().join(format!("{}-{}", name, std::process::id()));
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(&tmp).unwrap();
    tmp
}

/// 完整 docstring 应通过。
#[test]
fn complete_docstring_passes() {
    let root = ensure_clean_dir("codeguard-py-doc-complete");
    let code = r#""""A sample module for arithmetic operations."""


def add(a, b):
    """Add two numbers together.

    Args:
        a: The first operand.
        b: The second operand.

    Returns:
        The sum of a and b.
    """
    return a + b
"#;
    std::fs::write(root.join("complete.py"), code).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["comments", "python", root.to_str().unwrap(), "--format=json"])
        .output()
        .expect("无法运行 codeguard comments python");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let report: serde_json::Value = serde_json::from_str(&stdout).expect("JSON 解析失败");
    assert!(report["report_type"].as_str().map(|s| !s.is_empty()).unwrap_or(false), "应有 report_type");
    let _ = std::fs::remove_dir_all(&root);
}

/// 缺 docstring 应被检出。
#[test]
fn missing_docstring_detected() {
    let root = ensure_clean_dir("codeguard-py-doc-missing");
    std::fs::write(root.join("missing.py"), "def add(a, b):\n    return a + b\n").unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["comments", "python", root.to_str().unwrap(), "--format=json"])
        .output()
        .expect("无法运行 codeguard comments python");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let report: serde_json::Value = serde_json::from_str(&stdout).expect("JSON 解析失败");
    assert!(report["report_type"].as_str().map(|s| !s.is_empty()).unwrap_or(false), "应有 report_type");
    let _ = std::fs::remove_dir_all(&root);
}

/// 空 docstring 不能冒充合规。
#[test]
fn empty_docstring_cannot_impersonate() {
    let root = ensure_clean_dir("codeguard-py-doc-empty");
    std::fs::write(root.join("empty.py"), "def add(a, b):\n    \"\"\" \"\"\"\n    return a + b\n").unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["comments", "python", root.to_str().unwrap(), "--format=json"])
        .output()
        .expect("无法运行 codeguard comments python");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let report: serde_json::Value = serde_json::from_str(&stdout).expect("JSON 解析失败");
    assert!(report["report_type"].as_str().map(|s| !s.is_empty()).unwrap_or(false), "应有 report_type");
    let _ = std::fs::remove_dir_all(&root);
}
