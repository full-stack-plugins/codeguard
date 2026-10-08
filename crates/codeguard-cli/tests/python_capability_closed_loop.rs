//! Python 四能力真实闭环验收（15.6）。
//!
//! 验证检查→反馈→任务→修复→复检→关闭完整链路。

use std::process::Command;

fn ensure_clean_dir(name: &str) -> std::path::PathBuf {
    let tmp = std::env::temp_dir().join(format!("{}-{}", name, std::process::id()));
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(&tmp).unwrap();
    tmp
}

/// 检查→反馈→任务→修复→复检→关闭完整链路。
#[test]
fn check_feedback_task_fix_recheck_close() {
    let root = ensure_clean_dir("codeguard-python-closed-loop");

    // 1. 初始状态：干净代码
    std::fs::write(root.join("clean.py"), "def add(a, b):\n    return a + b\n").unwrap();

    // 2. 检查干净代码：应有类别反馈
    let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["check", "python", root.to_str().unwrap(), "--format=json"])
        .output()
        .expect("无法运行 codeguard check python");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let report: serde_json::Value = serde_json::from_str(&stdout).expect("JSON 解析失败");
    assert!(
        report["category_candidates"].as_array().is_some_and(|c| !c.is_empty()),
        "应有能力类别反馈"
    );

    // 3. 修复阶段：写入违规代码
    std::fs::write(root.join("broken.py"), "def f(:\n    pass\n").unwrap();

    // 4. 复检：有违规不得签发 allow
    let output2 = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["check", "python", root.to_str().unwrap(), "--format=json"])
        .output()
        .expect("无法运行 codeguard check python");
    let stdout2 = String::from_utf8_lossy(&output2.stdout);
    let report2: serde_json::Value = serde_json::from_str(&stdout2).expect("JSON 解析失败");
    assert_ne!(
        report2["delivery_decision"], "allow",
        "语法破损文件不得签发 allow"
    );
    assert_eq!(output2.status.code(), Some(3), "有发现时退出码应为 3");

    // 5. 关闭：修复后重检
    std::fs::write(root.join("broken.py"), "def f():\n    pass\n").unwrap();
    let output3 = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["check", "python", root.to_str().unwrap(), "--format=json"])
        .output()
        .expect("无法运行 codeguard check python");
    let stdout3 = String::from_utf8_lossy(&output3.stdout);
    let report3: serde_json::Value = serde_json::from_str(&stdout3).expect("JSON 解析失败");
    assert_ne!(
        report3["delivery_decision"], "deny",
        "修复后不得签发 deny"
    );

    let _ = std::fs::remove_dir_all(&root);
}

/// 四能力类别完整反馈。
#[test]
fn four_capabilities_complete_feedback() {
    let root = ensure_clean_dir("codeguard-python-four-caps");
    std::fs::write(root.join("test.py"), "x = 1\n").unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["check", "python", root.to_str().unwrap(), "--format=json"])
        .output()
        .expect("无法运行 codeguard check python");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let report: serde_json::Value = serde_json::from_str(&stdout).expect("JSON 解析失败");
    let candidates = report["category_candidates"]
        .as_array()
        .expect("category_candidates 应为数组");
    assert!(
        candidates.len() >= 4,
        "应覆盖至少 4 个核心能力类别，实际: {}",
        candidates.len()
    );
    for cat in candidates {
        assert!(
            cat["capability_status"].as_str().map(|s| !s.is_empty()).unwrap_or(false),
            "每个类别应有 capability_status: {}",
            cat["category"]
        );
    }
    let _ = std::fs::remove_dir_all(&root);
}

/// 重复检查幂等性。
#[test]
fn check_recheck_idempotent() {
    let root = ensure_clean_dir("codeguard-python-idempotent");
    std::fs::write(root.join("test.py"), "x = 1\n").unwrap();
    let out1 = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["check", "python", root.to_str().unwrap(), "--format=json"])
        .output()
        .expect("无法运行 codeguard check python");
    let r1: serde_json::Value =
        serde_json::from_str(&String::from_utf8_lossy(&out1.stdout)).expect("JSON 解析失败");
    let out2 = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["check", "python", root.to_str().unwrap(), "--format=json"])
        .output()
        .expect("无法运行 codeguard check python");
    let r2: serde_json::Value =
        serde_json::from_str(&String::from_utf8_lossy(&out2.stdout)).expect("JSON 解析失败");
    assert_eq!(
        r1["delivery_decision"], r2["delivery_decision"],
        "重复检查 delivery_decision 应幂等"
    );
    assert_eq!(out1.status.code(), out2.status.code(), "重复检查 exit_code 应幂等");
    let _ = std::fs::remove_dir_all(&root);
}
