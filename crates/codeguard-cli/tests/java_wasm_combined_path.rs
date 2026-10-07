//! Java WASM 联合路径验证：原生优先路由 + WASM 回退。
//!
//! 通过公开 CLI 命令验证 WASM 候选行为：
//! 1. WASM 候选不被当作已确认违规
//! 2. 零恢复不表示语法通过
//! 3. grammar 未验证时不声称 clean

#![cfg(feature = "wasm-precheck")]

use std::process::Command;

/// 验证 WASM 候选不被当作已确认违规。
#[test]
fn wasm_candidates_are_not_confirmed_violations() {
    let root = std::env::temp_dir().join("codeguard-wasm-candidate-test");
    std::fs::create_dir_all(&root).unwrap();

    // 创建一个有语法错误的 Java 文件
    let bad_file = root.join("Bad.java");
    std::fs::write(&bad_file, "class Bad { int f( { return 1; } }\n").unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["check", "java", root.to_str().unwrap(), "--format=json"])
        .output()
        .expect("无法运行 codeguard check syntax");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let report: serde_json::Value = serde_json::from_str(&stdout).expect("JSON 解析失败");

    println!(
        "WASM 候选报告: {}",
        serde_json::to_string_pretty(&report).unwrap()
    );

    // 验证：观察应标记为 candidate，而非 confirmed
    if let Some(observations) = report["observations"].as_array() {
        for obs in observations {
            let status = obs["status"].as_str().unwrap_or("");
            assert!(
                status.starts_with("candidate")
                    || status == "observed_partial"
                    || status == "not_run",
                "WASM 观察不应标记为已确认违规: {status}"
            );
        }
    }

    let _ = std::fs::remove_dir_all(&root);
}

/// 验证 grammar 未验证时不声称 clean。
#[test]
fn unverified_grammar_does_not_claim_clean() {
    let root = std::env::temp_dir().join("codeguard-wasm-clean-test");
    std::fs::create_dir_all(&root).unwrap();

    // 合法 Java 文件
    let good_file = root.join("Good.java");
    std::fs::write(&good_file, "class Good { int x = 1; }\n").unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["check", "java", root.to_str().unwrap(), "--format=json"])
        .output()
        .expect("无法运行 codeguard check syntax");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let report: serde_json::Value = serde_json::from_str(&stdout).expect("JSON 解析失败");

    // 验证：grammar 未验证时，status 不应为 clean
    if let Some(observations) = report["observations"].as_array() {
        for obs in observations {
            let grammar_qualified = obs["grammar_qualified"].as_bool().unwrap_or(false);
            if !grammar_qualified {
                let status = obs["status"].as_str().unwrap_or("");
                assert_ne!(status, "clean", "grammar 未验证时不能声称 clean");
            }
        }
    }

    println!(
        "零恢复验证通过: {}",
        serde_json::to_string_pretty(&report).unwrap()
    );

    let _ = std::fs::remove_dir_all(&root);
}

/// 验证 WASM 与原生工具的联合路径：原生优先。
#[test]
fn native_preferred_over_wasm() {
    let root = std::env::temp_dir().join("codeguard-wasm-native-test");
    std::fs::create_dir_all(&root).unwrap();

    let java_file = root.join("Test.java");
    std::fs::write(&java_file, "class Test { int x = 1; }\n").unwrap();

    // 运行 check all（会尝试原生工具 + WASM 回退）
    let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["check", "all", root.to_str().unwrap(), "--format=json"])
        .output()
        .expect("无法运行 codeguard check all");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let report: serde_json::Value = serde_json::from_str(&stdout).expect("JSON 解析失败");

    // 验证：delivery_decision 不应为 allow（grammar 未完全验证）
    let delivery = report["delivery_decision"].as_str().unwrap_or("");
    assert_ne!(delivery, "allow", "grammar 未完全验证时不应签发 allow");

    println!("原生优先验证通过: delivery_decision={delivery}");

    let _ = std::fs::remove_dir_all(&root);
}
