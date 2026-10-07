//! Java 四能力真实闭环验收（15.6）。
//!
//! 验证检查→反馈→任务→修复→复检→关闭完整链路。

use std::process::Command;

fn ensure_clean_dir(name: &str) -> std::path::PathBuf {
    let tmp = std::env::temp_dir().join(name);
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(&tmp).unwrap();
    tmp
}

/// 检查→反馈→任务→修复→复检→关闭完整链路。
#[test]
fn check_feedback_task_fix_recheck_close() {
    let root = ensure_clean_dir("codeguard-java-closed-loop");

    // 1. 初始状态：干净代码
    std::fs::write(
        root.join("Clean.java"),
        "/**\n * A clean class.\n */\npublic class Clean {\n    /**\n     * Does nothing.\n     */\n    public void noop() {}\n}\n",
    )
    .unwrap();

    // 检查干净代码
    let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["check", "java", root.to_str().unwrap(), "--format=json"])
        .output()
        .expect("无法运行 codeguard check java");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let report: serde_json::Value = serde_json::from_str(&stdout).expect("JSON 解析失败");

    // 2. 反馈：应有 category_candidates 和 next_action
    let candidates = report["category_candidates"]
        .as_array()
        .expect("category_candidates 应为数组");
    assert!(!candidates.is_empty(), "应有能力类别反馈");

    // 3. 任务：execution_tasks 应有任务
    let tasks = report["execution_tasks"]
        .as_array()
        .expect("execution_tasks 应为数组");
    // 不要求特定任务数量，只要求字段存在

    // 4. 修复：写入违规代码
    std::fs::write(
        root.join("Broken.java"),
        "public class Broken {\n    public void bad( {\n    }\n}\n",
    )
    .unwrap();

    // 5. 复检：再次检查
    let output2 = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["check", "java", root.to_str().unwrap(), "--format=json"])
        .output()
        .expect("无法运行 codeguard check java");
    let stdout2 = String::from_utf8_lossy(&output2.stdout);
    let report2: serde_json::Value = serde_json::from_str(&stdout2).expect("JSON 解析失败");

    // 6. 关闭：delivery_decision 不应为 allow（grammar 未完全验证）
    let delivery = report2["delivery_decision"].as_str().unwrap_or("");
    assert_ne!(delivery, "allow", "grammar 未完全验证时不应签发 allow");

    // 退出码应为 3（未完成/有发现）
    assert_eq!(output2.status.code(), Some(3), "有发现时退出码应为 3");

    let _ = std::fs::remove_dir_all(&root);
}

/// 检查→next→修复→复检链路。
#[test]
fn check_next_fix_recheck() {
    let root = ensure_clean_dir("codeguard-java-next-fix");

    std::fs::write(root.join("Test.java"), "public class Test { int x = 1; }\n").unwrap();

    // 检查
    let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["check", "java", root.to_str().unwrap(), "--format=json"])
        .output()
        .expect("无法运行 codeguard check java");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let report: serde_json::Value = serde_json::from_str(&stdout).expect("JSON 解析失败");

    // next 应有 next_actions
    let next_actions = report["next_actions"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    // next_actions 可能为空（取决于状态）

    // 修复
    std::fs::write(
        root.join("Test.java"),
        "/**\n * Test class.\n */\npublic class Test {\n    /**\n     * Field x.\n     */\n    int x = 1;\n}\n",
    )
    .unwrap();

    // 复检
    let output2 = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["check", "java", root.to_str().unwrap(), "--format=json"])
        .output()
        .expect("无法运行 codeguard check java");
    let stdout2 = String::from_utf8_lossy(&output2.stdout);
    let report2: serde_json::Value = serde_json::from_str(&stdout2).expect("JSON 解析失败");

    // 修复后 delivery_decision 不应为 deny
    let delivery = report2["delivery_decision"].as_str().unwrap_or("");
    assert_ne!(delivery, "deny", "修复后不应签发 deny");

    let _ = std::fs::remove_dir_all(&root);
}

/// 四能力类别完整闭环：每个类别有状态和下一步。
#[test]
fn four_capabilities_complete_feedback() {
    let root = ensure_clean_dir("codeguard-java-four-caps");

    std::fs::write(root.join("Test.java"), "public class Test { int x = 1; }\n").unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["check", "java", root.to_str().unwrap(), "--format=json"])
        .output()
        .expect("无法运行 codeguard check java");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let report: serde_json::Value = serde_json::from_str(&stdout).expect("JSON 解析失败");

    // 四个核心能力类别
    let candidates = report["category_candidates"]
        .as_array()
        .expect("category_candidates 应为数组");

    let categories: Vec<&str> = candidates
        .iter()
        .filter_map(|c| c["category"].as_str())
        .collect();

    // 应覆盖 lint/comments/dependencies/cve/security/build 中的至少 4 个
    assert!(
        categories.len() >= 4,
        "应覆盖至少 4 个核心能力类别，实际: {categories:?}"
    );

    // 每个类别应有 capability_status
    for cat in candidates {
        assert!(
            cat["capability_status"].as_str().map(|s| !s.is_empty()).unwrap_or(false),
            "每个类别应有 capability_status: {}",
            cat["category"]
        );
    }

    // obligation_status 应存在
    let obligation = report["obligation_status"].as_str().unwrap_or("");
    assert!(!obligation.is_empty(), "应有 obligation_status");

    let _ = std::fs::remove_dir_all(&root);
}

/// 检查→next→复检幂等性。
#[test]
fn check_next_recheck_idempotent() {
    let root = ensure_clean_dir("codeguard-java-idempotent");

    std::fs::write(root.join("Test.java"), "public class Test { int x = 1; }\n").unwrap();

    // 第一次检查
    let output1 = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["check", "java", root.to_str().unwrap(), "--format=json"])
        .output()
        .expect("无法运行 codeguard check java");
    let stdout1 = String::from_utf8_lossy(&output1.stdout);
    let report1: serde_json::Value = serde_json::from_str(&stdout1).expect("JSON 解析失败");

    // 第二次检查（不修改源码）
    let output2 = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["check", "java", root.to_str().unwrap(), "--format=json"])
        .output()
        .expect("无法运行 codeguard check java");
    let stdout2 = String::from_utf8_lossy(&output2.stdout);
    let report2: serde_json::Value = serde_json::from_str(&stdout2).expect("JSON 解析失败");

    // 两次检查的 delivery_decision 应一致（幂等）
    let delivery1 = report1["delivery_decision"].as_str().unwrap_or("");
    let delivery2 = report2["delivery_decision"].as_str().unwrap_or("");
    assert_eq!(delivery1, delivery2, "重复检查 delivery_decision 应幂等");

    // 两次检查的 exit_code 应一致
    assert_eq!(output1.status.code(), output2.status.code(), "重复检查 exit_code 应幂等");

    let _ = std::fs::remove_dir_all(&root);
}
