//! C/C++ 详细文档注释验收（15.3）。
//!
//! `comments <c|cpp> FILE --clang-tool ABS --standard` 使用真实 Clang
//! 文档观察档案。Clang 此档案不检查所有缺失注释（如实披露），
//! 本验收验证：完整文档原生完成零假阳性、符号链接拒绝、
//! 坏工具拒绝、报告结构与复检命令完整。

use std::process::Command;

fn ensure_clean_dir(name: &str) -> std::path::PathBuf {
    let tmp = std::env::temp_dir().join(format!("{}-{}", name, std::process::id()));
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(&tmp).unwrap();
    // 解析符号链接：Clang 源路径安全检查拒绝未解析路径
    tmp.canonicalize().unwrap_or(tmp)
}

fn clang_tool() -> &'static str {
    "/usr/bin/clang"
}

fn clangpp_tool() -> &'static str {
    "/usr/bin/clang++"
}

const COMPLETE_C: &str = r#"/**
 * @file complete.c
 * @brief A sample module for arithmetic operations.
 */

/**
 * @brief Adds two numbers together.
 *
 * @param a The first operand.
 * @param b The second operand.
 * @return The sum of a and b.
 */
int add(int a, int b) {
    return a + b;
}
"#;

const COMPLETE_CPP: &str = r#"/**
 * @file complete.cpp
 * @brief A sample class for arithmetic operations.
 */

/**
 * @brief Adds two numbers together.
 *
 * @param a The first operand.
 * @param b The second operand.
 * @return The sum of a and b.
 */
int add(int a, int b) {
    return a + b;
}
"#;

/// 完整 Doxygen 文档的 C 文件：真实 Clang 原生扫描完成且零假阳性。
#[test]
fn complete_c_documentation_native_scan_no_false_positive() {
    let root = ensure_clean_dir("codeguard-c-doc-complete");
    let file = root.join("complete.c");
    std::fs::write(&file, COMPLETE_C).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args([
            "comments",
            "c",
            file.to_str().unwrap(),
            "--clang-tool",
            clang_tool(),
            "--standard",
            "c11",
            "--format=json",
        ])
        .output()
        .expect("无法运行 codeguard comments c");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let report: serde_json::Value = serde_json::from_str(&stdout).expect("JSON 解析失败");

    // 原生扫描应完成（真实 Clang 被调用）
    assert_eq!(
        report["native"]["status"], "completed",
        "完整文档的 C 文件原生扫描应完成: {}",
        report["native"]["reason"]
    );
    // 完整文档不得产生原生诊断（零假阳性）
    let diagnostics = report["native"]["diagnostics"].as_array().cloned().unwrap_or_default();
    assert!(
        diagnostics.is_empty(),
        "完整 Doxygen 文档不应有原生诊断（零假阳性）: {diagnostics:?}"
    );
    // 报告不得声称 clean（详细文档合规需完整政策）
    assert_ne!(
        report["delivery_decision"], "allow",
        "详细文档合规政策未完成，不得签发 allow"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// 完整 Doxygen 文档的 C++ 文件：真实 Clang++ 原生扫描完成且零假阳性。
#[test]
fn complete_cpp_documentation_native_scan_no_false_positive() {
    let root = ensure_clean_dir("codeguard-cpp-doc-complete");
    let file = root.join("complete.cpp");
    std::fs::write(&file, COMPLETE_CPP).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args([
            "comments",
            "cpp",
            file.to_str().unwrap(),
            "--clang-tool",
            clangpp_tool(),
            "--standard",
            "c++17",
            "--format=json",
        ])
        .output()
        .expect("无法运行 codeguard comments cpp");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let report: serde_json::Value = serde_json::from_str(&stdout).expect("JSON 解析失败");

    assert_eq!(
        report["native"]["status"], "completed",
        "完整文档的 C++ 文件原生扫描应完成: {}",
        report["native"]["reason"]
    );
    let diagnostics = report["native"]["diagnostics"].as_array().cloned().unwrap_or_default();
    assert!(
        diagnostics.is_empty(),
        "完整 Doxygen 文档不应有原生诊断（零假阳性）: {diagnostics:?}"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// 符号链接源路径应被明确拒绝，不得静默扫描。
#[test]
fn symlink_source_path_is_rejected() {
    let root = ensure_clean_dir("codeguard-c-doc-symlink");
    let file = root.join("real.c");
    std::fs::write(&file, COMPLETE_C).unwrap();
    let link = root.join("link.c");
    std::os::unix::fs::symlink(&file, &link).unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args([
            "comments",
            "c",
            link.to_str().unwrap(),
            "--clang-tool",
            clang_tool(),
            "--standard",
            "c11",
            "--format=json",
        ])
        .output()
        .expect("无法运行 codeguard comments c");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let report: serde_json::Value = serde_json::from_str(&stdout).expect("JSON 解析失败");

    assert_eq!(
        report["native"]["reason"], "source_path_symlink_disallowed",
        "符号链接源路径应被明确拒绝"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// 不存在的工具路径应被明确拒绝，不得伪造原生完成。
#[test]
fn missing_tool_is_explicitly_rejected() {
    let root = ensure_clean_dir("codeguard-c-doc-badtool");
    let file = root.join("complete.c");
    std::fs::write(&file, COMPLETE_C).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args([
            "comments",
            "c",
            file.to_str().unwrap(),
            "--clang-tool",
            "/nonexistent/clang/path",
            "--standard",
            "c11",
            "--format=json",
        ])
        .output()
        .expect("无法运行 codeguard comments c");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let report: serde_json::Value = serde_json::from_str(&stdout).expect("JSON 解析失败");

    // 坏工具不得声称原生完成
    assert_ne!(
        report["native"]["status"], "completed",
        "不存在的工具路径不得伪造原生完成"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// 报告应包含可复现的复检命令与固定标准。
#[test]
fn report_contains_verification_command_and_standard() {
    let root = ensure_clean_dir("codeguard-c-doc-verify-cmd");
    let file = root.join("complete.c");
    std::fs::write(&file, COMPLETE_C).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args([
            "comments",
            "c",
            file.to_str().unwrap(),
            "--clang-tool",
            clang_tool(),
            "--standard",
            "c11",
            "--format=json",
        ])
        .output()
        .expect("无法运行 codeguard comments c");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let report: serde_json::Value = serde_json::from_str(&stdout).expect("JSON 解析失败");

    // 报告类型与标准
    assert_eq!(report["report_type"], "c_family_comments_feedback");
    assert_eq!(
        report["documentation_configuration"]["standard"], "c11",
        "报告应固定声明语言标准"
    );
    // 复检命令应包含原工具与标准参数
    let verify: Vec<String> = report["verification_command"]
        .as_array()
        .cloned()
        .unwrap_or_default()
        .iter()
        .filter_map(|v| v.as_str().map(str::to_owned))
        .collect();
    let joined = verify.join(" ");
    assert!(
        joined.contains("--clang-tool") && joined.contains("--standard"),
        "复检命令应包含原工具与标准: {joined}"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// 缺文档样本：Clang 档案如实不检查所有缺失注释，
/// 报告不得声称 clean，须保留修复指引。
#[test]
fn missing_docs_keeps_honest_incomplete_status() {
    let root = ensure_clean_dir("codeguard-c-doc-missing");
    let file = root.join("missing.c");
    std::fs::write(&file, "int add(int a, int b) {\n    return a + b;\n}\n").unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args([
            "comments",
            "c",
            file.to_str().unwrap(),
            "--clang-tool",
            clang_tool(),
            "--standard",
            "c11",
            "--format=json",
        ])
        .output()
        .expect("无法运行 codeguard comments c");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let report: serde_json::Value = serde_json::from_str(&stdout).expect("JSON 解析失败");

    // 原生扫描应完成
    assert_eq!(report["native"]["status"], "completed");
    // 但命令整体不得声称完成（零诊断不能证明详细文档合规）
    assert_eq!(report["command_status"], "incomplete");
    assert_ne!(report["delivery_decision"], "allow");
    // 应有修复指引
    let actions = report["next_actions"].as_array().cloned().unwrap_or_default();
    assert!(
        !actions.is_empty(),
        "缺文档样本应保留下一步指引，不得静默放行"
    );
    let _ = std::fs::remove_dir_all(&root);
}
