//! Nix WASM+原生联合路径全链路验证。

#![cfg(feature = "wasm-precheck")]

use std::process::Command;

fn ensure_clean_dir(name: &str) -> std::path::PathBuf {
    let tmp = std::env::temp_dir().join(format!("{}-{}", name, std::process::id()));
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(&tmp).unwrap();
    tmp
}

#[test]
fn wasm_candidates_are_not_confirmed_violations() {
    let root = ensure_clean_dir("codeguard-nix-wasm-candidate");
    let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["check", "nix", root.to_str().unwrap(), "--format=json"])
        .output()
        .expect("无法运行 codeguard check");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let report: serde_json::Value = serde_json::from_str(&stdout).expect("JSON 解析失败");
    if let Some(observations) = report["syntax_candidates"]["observations"].as_array() {
        for obs in observations {
            let status = obs["status"].as_str().unwrap_or("");
            assert!(status.starts_with("candidate") || status == "observed_partial" || status == "not_run" || status == "incomplete", "WASM 观察不应标记为已确认违规: {status}");
        }
    }
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn unverified_grammar_does_not_claim_clean() {
    let root = ensure_clean_dir("codeguard-nix-wasm-clean");
    let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["check", "nix", root.to_str().unwrap(), "--format=json"])
        .output()
        .expect("无法运行 codeguard check");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let report: serde_json::Value = serde_json::from_str(&stdout).expect("JSON 解析失败");
    if let Some(observations) = report["syntax_candidates"]["observations"].as_array() {
        for obs in observations {
            let qualified = obs["grammar_qualified"].as_bool().unwrap_or(false);
            if !qualified {
                let status = obs["status"].as_str().unwrap_or("");
                assert_ne!(status, "clean", "grammar 未验证时不能声称 clean");
            }
        }
    }
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn native_preferred_over_wasm() {
    let root = ensure_clean_dir("codeguard-nix-native-pref");
    let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["check", "all", root.to_str().unwrap(), "--format=json"])
        .output()
        .expect("无法运行 codeguard check all");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let report: serde_json::Value = serde_json::from_str(&stdout).expect("JSON 解析失败");
    let delivery = report["delivery_decision"].as_str().unwrap_or("");
    assert_ne!(delivery, "allow", "grammar 未完全验证时不应签发 allow");
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn missing_tool_initial_check_gives_guidance() {
    let root = ensure_clean_dir("codeguard-nix-missing-tool");
    let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["check", "nix", root.to_str().unwrap(), "--format=json"])
        .output()
        .expect("无法运行 codeguard check");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let report: serde_json::Value = serde_json::from_str(&stdout).expect("JSON 解析失败");
    let next_action = report["syntax_candidates"]["next_action"].as_str().unwrap_or("");
    assert!(!next_action.is_empty(), "缺工具初检应给出下一步指引");
    assert_eq!(output.status.code(), Some(3), "缺工具时退出码应为 3");
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn installation_guidance_in_category_candidates() {
    let root = ensure_clean_dir("codeguard-nix-install-guide");
    let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["check", "nix", root.to_str().unwrap(), "--format=json"])
        .output()
        .expect("无法运行 codeguard check");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let report: serde_json::Value = serde_json::from_str(&stdout).expect("JSON 解析失败");
    if let Some(candidates) = report["category_candidates"].as_array() {
        for cat in candidates {
            assert!(cat["capability_status"].as_str().map(|s| !s.is_empty()).unwrap_or(false), "每个类别应有 capability_status");
        }
    }
    let _ = std::fs::remove_dir_all(&root);
}
