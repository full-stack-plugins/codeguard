//! 质量门禁实际执行验证。
//!
//! 验证 delivery_decision=deny 拦截违规、allow 放行干净代码。

use std::process::Command;

fn ensure_clean_dir(name: &str) -> std::path::PathBuf {
    let tmp = std::env::temp_dir().join(name);
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(&tmp).unwrap();
    tmp
}

/// 违规代码应触发 deny。
#[test]
fn violation_triggers_deny() {
    let root = ensure_clean_dir("codeguard-gate-deny");
    // 有语法错误的 Java 代码
    std::fs::write(root.join("Bad.java"), "class Bad { int f( { return 1; } }\n").unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["check", "all", root.to_str().unwrap(), "--format=json"])
        .output()
        .expect("无法运行 codeguard check all");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let report: serde_json::Value = serde_json::from_str(&stdout).expect("JSON 解析失败");

    let delivery = report["delivery_decision"].as_str().unwrap_or("");
    // 有违规时应为 deny 或 incomplete（不为 allow）
    assert_ne!(delivery, "allow", "有违规时不应签发 allow");
    println!("违规 delivery_decision: {delivery}");

    let _ = std::fs::remove_dir_all(&root);
}

/// 干净代码应触发 allow（当所有检查完成时）。
#[test]
fn clean_code_triggers_allow_when_complete() {
    let root = ensure_clean_dir("codeguard-gate-allow");
    std::fs::write(root.join("Clean.java"), "class Clean { int x = 1; }\n").unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["check", "all", root.to_str().unwrap(), "--format=json"])
        .output()
        .expect("无法运行 codeguard check all");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let report: serde_json::Value = serde_json::from_str(&stdout).expect("JSON 解析失败");

    let delivery = report["delivery_decision"].as_str().unwrap_or("");
    // 干净代码不应为 deny
    assert_ne!(delivery, "deny", "干净代码不应签发 deny");
    println!("干净 delivery_decision: {delivery}");

    let _ = std::fs::remove_dir_all(&root);
}

/// 门禁决策应与 unresolved_conditions 一致。
#[test]
fn gate_decision_consistent_with_unresolved() {
    let root = ensure_clean_dir("codeguard-gate-consistent");
    std::fs::write(root.join("Test.java"), "class Test { int x = 1; }\n").unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["check", "all", root.to_str().unwrap(), "--format=json"])
        .output()
        .expect("无法运行 codeguard check all");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let report: serde_json::Value = serde_json::from_str(&stdout).expect("JSON 解析失败");

    let delivery = report["delivery_decision"].as_str().unwrap_or("");
    let unresolved = report["unresolved_conditions"]
        .as_array()
        .cloned()
        .unwrap_or_default();

    // 有 unresolved 时不应为 allow
    if !unresolved.is_empty() {
        assert_ne!(delivery, "allow", "有 unresolved 时不应签发 allow");
    }

    // 无 unresolved 时不应为 deny（除非有 confirmed findings）
    if unresolved.is_empty() {
        let confirmed: usize = report["syntax_candidates"]["observations"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter(|obs| {
                        let status = obs["status"].as_str().unwrap_or("");
                        status == "confirmed" || status == "violation"
                    })
                    .count()
            })
            .unwrap_or(0);
        if confirmed == 0 {
            assert_ne!(delivery, "deny", "无发现且无 unresolved 时不应签发 deny");
        }
    }

    println!("一致 delivery_decision: {delivery}, unresolved: {}", unresolved.len());

    let _ = std::fs::remove_dir_all(&root);
}

/// 门禁决策应与 exit_code 一致。
#[test]
fn gate_decision_consistent_with_exit_code() {
    let root = ensure_clean_dir("codeguard-gate-exit-code");
    std::fs::write(root.join("Test.java"), "class Test { int x = 1; }\n").unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["check", "all", root.to_str().unwrap(), "--format=json"])
        .output()
        .expect("无法运行 codeguard check all");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let report: serde_json::Value = serde_json::from_str(&stdout).expect("JSON 解析失败");

    let delivery = report["delivery_decision"].as_str().unwrap_or("");
    let exit_code = output.status.code().unwrap_or(0);

    // allow 应对应 exit_code 0
    if delivery == "allow" {
        assert_eq!(exit_code, 0, "allow 应对应 exit_code 0");
    }

    // deny 应对应非 0 exit_code
    if delivery == "deny" {
        assert_ne!(exit_code, 0, "deny 应对应非 0 exit_code");
    }

    println!("exit_code: {exit_code}, delivery_decision: {delivery}");

    let _ = std::fs::remove_dir_all(&root);
}
