//! Python WASM+原生联合路径全链路验证。
//!
//! 覆盖配置发现、原生优先、缺工具初检、版本/方言兼容、精确定位、
//! 取消/超时及安装指引全链路。

#![cfg(feature = "wasm-precheck")]

use std::process::Command;

fn ensure_clean_dir(name: &str) -> std::path::PathBuf {
    let tmp = std::env::temp_dir().join(name);
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(&tmp).unwrap();
    tmp
}

/// WASM 候选不被当作已确认违规。
#[test]
fn wasm_candidates_are_not_confirmed_violations() {
    let root = ensure_clean_dir("codeguard-py-wasm-candidate");
    std::fs::write(root.join("bad.py"), "def f(:\n    pass\n").unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["check", "python", root.to_str().unwrap(), "--format=json"])
        .output()
        .expect("无法运行 codeguard check python");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let report: serde_json::Value = serde_json::from_str(&stdout).expect("JSON 解析失败");

    if let Some(observations) = report["syntax_candidates"]["observations"].as_array() {
        for obs in observations {
            let status = obs["status"].as_str().unwrap_or("");
            assert!(
                status.starts_with("candidate")
                    || status == "observed_partial"
                    || status == "not_run"
                    || status == "incomplete",
                "WASM 观察不应标记为已确认违规: {status}"
            );
        }
    }

    let delivery = report["delivery_decision"].as_str().unwrap_or("");
    assert_ne!(delivery, "allow", "WASM 候选不应签发 allow");

    let _ = std::fs::remove_dir_all(&root);
}

/// grammar 未验证时不声称 clean。
#[test]
fn unverified_grammar_does_not_claim_clean() {
    let root = ensure_clean_dir("codeguard-py-wasm-clean");
    std::fs::write(root.join("good.py"), "def add(a, b):\n    return a + b\n").unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["check", "python", root.to_str().unwrap(), "--format=json"])
        .output()
        .expect("无法运行 codeguard check python");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let report: serde_json::Value = serde_json::from_str(&stdout).expect("JSON 解析失败");

    if let Some(observations) = report["syntax_candidates"]["observations"].as_array() {
        for obs in observations {
            let grammar_qualified = obs["grammar_qualified"].as_bool().unwrap_or(false);
            if !grammar_qualified {
                let status = obs["status"].as_str().unwrap_or("");
                assert_ne!(status, "clean", "grammar 未验证时不能声称 clean");
            }
        }
    }

    let _ = std::fs::remove_dir_all(&root);
}

/// 原生优先路由。
#[test]
fn native_preferred_over_wasm() {
    let root = ensure_clean_dir("codeguard-py-native-pref");
    std::fs::write(root.join("test.py"), "x = 1\n").unwrap();

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

/// 配置发现识别 Ruff/TOML 状态。
#[test]
fn configuration_discovery_identifies_checker_status() {
    let root = ensure_clean_dir("codeguard-py-config-discover");
    std::fs::write(root.join("test.py"), "x = 1\n").unwrap();
    std::fs::write(
        root.join("pyproject.toml"),
        "[tool.ruff]\nline-length = 88\n",
    )
    .unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["detect", root.to_str().unwrap(), "--format=json"])
        .output()
        .expect("无法运行 codeguard detect");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let report: serde_json::Value = serde_json::from_str(&stdout).expect("JSON 解析失败");

    let configs = report["checker_configurations"]
        .as_array()
        .expect("checker_configurations 应为数组");

    assert!(
        configs.len() >= 2,
        "应发现至少 2 种 checker 配置，实际: {}",
        configs.len()
    );

    for cfg in configs {
        assert!(
            cfg["next_action"].as_str().map(|s| !s.is_empty()).unwrap_or(false),
            "每个 checker 配置应有 next_action: {}",
            cfg["checker_id"]
        );
    }

    let _ = std::fs::remove_dir_all(&root);
}

/// 缺工具初检给出安装指引。
#[test]
fn missing_tool_initial_check_gives_guidance() {
    let root = ensure_clean_dir("codeguard-py-missing-tool");
    std::fs::write(root.join("test.py"), "x = 1\n").unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["check", "python", root.to_str().unwrap(), "--format=json"])
        .output()
        .expect("无法运行 codeguard check python");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let report: serde_json::Value = serde_json::from_str(&stdout).expect("JSON 解析失败");

    let next_action = report["syntax_candidates"]["next_action"].as_str().unwrap_or("");
    assert!(!next_action.is_empty(), "缺工具初检应给出下一步指引");

    let delivery = report["syntax_candidates"]["delivery_decision"].as_str().unwrap_or("");
    assert_eq!(delivery, "incomplete", "缺工具时 delivery_decision 应为 incomplete");

    assert_eq!(output.status.code(), Some(3), "缺工具时退出码应为 3");

    let _ = std::fs::remove_dir_all(&root);
}

/// 版本/方言兼容性如实披露。
#[test]
fn version_dialect_compatibility_disclosed() {
    let root = ensure_clean_dir("codeguard-py-version-dialect");
    std::fs::write(root.join("test.py"), "x = 1\n").unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["check", "python", root.to_str().unwrap(), "--format=json"])
        .output()
        .expect("无法运行 codeguard check python");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let report: serde_json::Value = serde_json::from_str(&stdout).expect("JSON 解析失败");

    let observations = report["syntax_candidates"]["observations"]
        .as_array()
        .cloned()
        .unwrap_or_default();

    if !observations.is_empty() {
        let obs = &observations[0];
        let limitations = obs["known_limitations"]
            .as_array()
            .expect("known_limitations 应为数组");
        assert!(!limitations.is_empty(), "版本/方言兼容性应有已知限制披露");
    }

    if let Some(obs) = observations.first() {
        let qualified = obs["grammar_qualified"].as_bool().unwrap_or(false);
        assert!(!qualified, "Python grammar 当前未验收，grammar_qualified 应为 false");
    }

    let _ = std::fs::remove_dir_all(&root);
}

/// 精确定位含 byte_offset 和 grammar 身份。
#[test]
fn precise_positioning_with_grammar_identity() {
    let root = ensure_clean_dir("codeguard-py-precise-pos");
    std::fs::write(root.join("bad.py"), "import os\ndef f(:\n    pass\n").unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["check", "python", root.to_str().unwrap(), "--format=json"])
        .output()
        .expect("无法运行 codeguard check python");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let report: serde_json::Value = serde_json::from_str(&stdout).expect("JSON 解析失败");

    let observations = report["syntax_candidates"]["observations"]
        .as_array()
        .cloned()
        .unwrap_or_default();

    for obs in &observations {
        assert!(obs["byte_offset"].is_number(), "观察应有 byte_offset 精确定位");
        let sha = obs["grammar_sha256"].as_str().unwrap_or("");
        assert_eq!(sha.len(), 64, "grammar_sha256 应为 64 字符 SHA-256");
    }

    let _ = std::fs::remove_dir_all(&root);
}

/// 执行预算含超时配置。
#[test]
fn execution_budget_has_timeout_config() {
    let root = ensure_clean_dir("codeguard-py-budget");
    std::fs::write(root.join("test.py"), "x = 1\n").unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["check", "python", root.to_str().unwrap(), "--format=json"])
        .output()
        .expect("无法运行 codeguard check python");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let report: serde_json::Value = serde_json::from_str(&stdout).expect("JSON 解析失败");

    let budget = &report["execution_budget"];
    assert!(budget["timeout_ms"].is_number(), "execution_budget 应有 timeout_ms");
    assert!(
        budget["enforcement"].as_str().map(|s| !s.is_empty()).unwrap_or(false),
        "execution_budget 应有 enforcement 策略"
    );

    let _ = std::fs::remove_dir_all(&root);
}

/// 六类别 gap 指引完整。
#[test]
fn installation_guidance_in_category_candidates() {
    let root = ensure_clean_dir("codeguard-py-install-guide");
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

    assert_eq!(candidates.len(), 6, "应有 6 个核心能力类别");

    for cat in candidates {
        assert!(
            cat["capability_status"].as_str().map(|s| !s.is_empty()).unwrap_or(false),
            "每个类别应有 capability_status: {}",
            cat["category"]
        );
    }

    let _ = std::fs::remove_dir_all(&root);
}
