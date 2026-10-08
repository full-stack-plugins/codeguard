//! TypeScript 详细文档注释验收（15.3）。
//!
//! 当前 `comments` 命令仅支持 rust/java/python/c/cpp。
//! TypeScript 文档注释按规范如实披露为未集成——命令明确拒绝而非静默成功。

use std::process::Command;

fn ensure_clean_dir(name: &str) -> std::path::PathBuf {
    let tmp = std::env::temp_dir().join(format!("{}-{}", name, std::process::id()));
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(&tmp).unwrap();
    tmp
}

/// 不支持的语言应被明确拒绝，不得伪造 JSON 报告。
#[test]
fn unsupported_language_is_explicitly_rejected() {
    let root = ensure_clean_dir("codeguard-ts-doc-unsupported");
    let code = "/**\n * Adds two numbers together.\n * @param a - The first operand.\n * @param b - The second operand.\n * @returns The sum of a and b.\n */\nexport function add(a: number, b: number): number {\n    return a + b;\n}\n";
    std::fs::write(root.join("complete.ts"), code).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["comments", "typescript", root.to_str().unwrap(), "--format=json"])
        .output()
        .expect("无法运行 codeguard comments typescript");
    // 不支持的语言必须显式拒绝（非 0 退出码），不得静默成功
    assert_ne!(
        output.status.code(),
        Some(0),
        "TypeScript comments 未集成，应显式拒绝而非静默成功"
    );
    // 输出不得伪造 JSON 报告冒充合规
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.trim().is_empty() || !stdout.contains("\"report_type\""),
        "未集成的语言不得伪造报告: {stdout}"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// check typescript 的 comments 类别应如实披露 not_integrated。
#[test]
fn comments_category_disclosed_as_not_integrated() {
    let root = ensure_clean_dir("codeguard-ts-doc-category");
    std::fs::write(root.join("test.ts"), "export const x: number = 1;\n").unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["check", "typescript", root.to_str().unwrap(), "--format=json"])
        .output()
        .expect("无法运行 codeguard check typescript");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let report: serde_json::Value = serde_json::from_str(&stdout).expect("JSON 解析失败");
    if let Some(candidates) = report["category_candidates"].as_array() {
        if let Some(comments) = candidates.iter().find(|c| c["category"] == "comments") {
            let status = comments["status"].as_str().unwrap_or("");
            // 未集成的 comments 类别不得声称已验证
            assert_ne!(
                status,
                "verified",
                "TypeScript comments 未集成，类别状态不得为 verified: {status}"
            );
        }
    }
    let _ = std::fs::remove_dir_all(&root);
}
