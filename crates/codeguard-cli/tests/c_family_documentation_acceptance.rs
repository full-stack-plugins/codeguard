//! C/C++ 详细文档注释验收（15.3）。
//!
//! 使用显式固定协议的受控子进程，验证 CLI 观察与报告契约。
//! 这些样本不证明真实 Clang 的文档解析准确性；真实工具回放保留在
//! c_family_comments_cli 的显式 Apple Clang 21 测试中。
//! 同时验证错版本在扫描前被拒绝，零诊断不能证明详细文档合规。

use std::{os::unix::fs::PermissionsExt, path::Path, process::Command};

fn ensure_clean_dir(name: &str) -> std::path::PathBuf {
    let tmp = std::env::temp_dir().join(format!("{}-{}", name, std::process::id()));
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(&tmp).unwrap();
    // 解析符号链接：Clang 源路径安全检查拒绝未解析路径
    tmp.canonicalize().unwrap_or(tmp)
}

/// 受控协议夹具：不能把 CI 预装的任意 Clang 冒充固定 Apple Clang。
fn clang_tool(root: &Path) -> String {
    clang_with_version(root, "Apple clang version 21.0.0 (clang-2100.3.34.2)")
}

fn clang_with_version(root: &Path, version: &str) -> String {
    let tool = root.join("clang-fixture");
    let report = serde_json::json!({
        "version":"2.1.0",
        "runs":[{"columnKind":"unicodeCodePoints",
            "invocations":[{"executionSuccessful":true}], "artifacts":[],
            "tool":{"driver":{"name":"clang",
                "version":"Apple clang version 21.0.0 (clang-2100.3.34.2)","rules":[]}},
            "results":[]}]
    });
    std::fs::write(&tool, format!(
        "#!/bin/sh\nif [ \"$1\" = '--version' ]; then echo '{version}'; exit 0; fi\ncat >/dev/null\ntouch '{}/scan-executed'\ncat <<'REPORT' >&2\n{report}\nREPORT\n",
        root.display()
    )).unwrap();
    std::fs::set_permissions(&tool, std::fs::Permissions::from_mode(0o700)).unwrap();
    tool.to_str().unwrap().to_owned()
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

/// C 文档样本的受控零诊断回执保留本地完成状态，不签发合规结论。
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
            &clang_tool(&root),
            "--standard",
            "c11",
            "--format=json",
        ])
        .output()
        .expect("无法运行 codeguard comments c");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let report: serde_json::Value = serde_json::from_str(&stdout).expect("JSON 解析失败");

    // 受控子进程报告完成，不冒充真实工具准确性验收。
    assert_eq!(
        report["native"]["status"], "completed",
        "完整文档的 C 文件原生扫描应完成: {}",
        report["native"]["reason"]
    );
    assert_eq!(report["detailed_contract_qualification"], "not_granted");
    // 协议夹具的空诊断必须原样保留，不冒充真实语义证明。
    assert!(root.join("scan-executed").exists());
    let diagnostics = report["native"]["diagnostics"]
        .as_array()
        .cloned()
        .unwrap_or_default();
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

/// C++ 文档样本的受控零诊断回执保留本地完成状态。
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
            &clang_tool(&root),
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
    assert!(root.join("scan-executed").exists());
    let diagnostics = report["native"]["diagnostics"]
        .as_array()
        .cloned()
        .unwrap_or_default();
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
            &clang_tool(&root),
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
            &clang_tool(&root),
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
            &clang_tool(&root),
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
    assert!(root.join("scan-executed").exists());
    // 但命令整体不得声称完成（零诊断不能证明详细文档合规）
    assert_eq!(report["command_status"], "incomplete");
    assert_ne!(report["delivery_decision"], "allow");
    // 应有修复指引
    let actions = report["next_actions"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    assert!(
        !actions.is_empty(),
        "缺文档样本应保留下一步指引，不得静默放行"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// 相近版本或另一发行版均不能获得固定检查器资格，也不得开始扫描。
#[test]
fn unverified_clang_versions_are_rejected_before_scanning() {
    for (name, version) in [
        ("upstream", "clang version 21.0.0"),
        ("apple-other", "Apple clang version 20.0.0 (clang-2000.0.0)"),
    ] {
        let root = ensure_clean_dir(&format!("codeguard-c-doc-version-{name}"));
        let file = root.join("complete.c");
        std::fs::write(&file, COMPLETE_C).unwrap();
        let tool = clang_with_version(&root, version);
        let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
            .args([
                "comments",
                "c",
                file.to_str().unwrap(),
                "--clang-tool",
                &tool,
                "--standard",
                "c11",
                "--format=json",
            ])
            .output()
            .unwrap();
        let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(report["native"]["status"], "incomplete");
        assert_eq!(report["native"]["reason"], "clang_version_unverified");
        assert_ne!(report["delivery_decision"], "allow");
        assert!(!root.join("scan-executed").exists());
        std::fs::remove_dir_all(root).unwrap();
    }
}
