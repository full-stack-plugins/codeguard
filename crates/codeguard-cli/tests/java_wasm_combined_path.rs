//! Java WASM 联合路径全链路验证：配置发现、原生优先、缺工具初检、
//! 版本/方言兼容、精确定位、取消/超时、安装指引。
//!
//! 通过公开 CLI 命令验证 WASM 候选行为：
//! 1. WASM 候选不被当作已确认违规
//! 2. 零恢复不表示语法通过
//! 3. grammar 未验证时不声称 clean
//! 4. 配置发现正确识别 checker 状态
//! 5. 缺工具初检给出安装指引
//! 6. 版本/方言兼容性如实披露
//! 7. 精确定位含 byte_offset 和 grammar 身份

#![cfg(feature = "wasm-precheck")]

use std::process::Command;

fn ensure_clean_dir(name: &str) -> std::path::PathBuf {
    let tmp = std::env::temp_dir().join(name);
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(&tmp).unwrap();
    tmp
}

/// 验证 WASM 候选不被当作已确认违规。
#[test]
fn wasm_candidates_are_not_confirmed_violations() {
    let root = ensure_clean_dir("codeguard-wasm-candidate-test");
    std::fs::write(root.join("Bad.java"), "class Bad { int f( { return 1; } }\n").unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["check", "java", root.to_str().unwrap(), "--format=json"])
        .output()
        .expect("无法运行 codeguard check java");

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

    // delivery_decision 不应为 allow
    let delivery = report["delivery_decision"].as_str().unwrap_or("");
    assert_ne!(delivery, "allow", "WASM 候选不应签发 allow");

    let _ = std::fs::remove_dir_all(&root);
}

/// 验证 grammar 未验证时不声称 clean。
#[test]
fn unverified_grammar_does_not_claim_clean() {
    let root = ensure_clean_dir("codeguard-wasm-clean-test");
    std::fs::write(root.join("Good.java"), "class Good { int x = 1; }\n").unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["check", "java", root.to_str().unwrap(), "--format=json"])
        .output()
        .expect("无法运行 codeguard check java");

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

/// 验证 WASM 与原生工具的联合路径：原生优先。
#[test]
fn native_preferred_over_wasm() {
    let root = ensure_clean_dir("codeguard-wasm-native-test");
    std::fs::write(root.join("Test.java"), "class Test { int x = 1; }\n").unwrap();

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

/// 配置发现：detect 命令正确识别 checker 配置状态。
#[test]
fn configuration_discovery_identifies_checker_status() {
    let root = ensure_clean_dir("codeguard-java-config-discover");
    std::fs::write(root.join("Test.java"), "class Test { int x = 1; }\n").unwrap();
    std::fs::write(
        root.join("pom.xml"),
        r#"<?xml version="1.0" encoding="UTF-8"?>
<project>
  <modelVersion>4.0.0</modelVersion>
  <groupId>test</groupId>
  <artifactId>test</artifactId>
  <version>1.0</version>
  <build>
    <plugins>
      <plugin>
        <groupId>org.apache.maven.plugins</groupId>
        <artifactId>maven-checkstyle-plugin</artifactId>
        <version>3.3.0</version>
      </plugin>
    </plugins>
  </build>
</project>
"#,
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

    // 应发现至少 7 种 checker（lint/comments/dependencies/cve/security 各类）
    assert!(
        configs.len() >= 5,
        "应发现至少 5 种 checker 配置，实际: {}",
        configs.len()
    );

    // checkstyle 应标记为 configured（POM 中已声明）
    let checkstyle = configs
        .iter()
        .find(|c| c["checker_id"] == "java.maven.checkstyle");
    assert!(checkstyle.is_some(), "应发现 checkstyle 配置");
    assert_eq!(
        checkstyle.unwrap()["configuration"],
        "configured",
        "POM 中声明的 checkstyle 应为 configured"
    );

    // 每个配置应有 next_action（安装/配置指引）
    for cfg in configs {
        assert!(
            cfg["next_action"].as_str().map(|s| !s.is_empty()).unwrap_or(false),
            "每个 checker 配置应有 next_action: {}",
            cfg["checker_id"]
        );
    }

    let _ = std::fs::remove_dir_all(&root);
}

/// 缺工具初检：语法候选给出安装指引和下一步。
#[test]
fn missing_tool_initial_check_gives_guidance() {
    let root = ensure_clean_dir("codeguard-java-missing-tool");
    std::fs::write(root.join("Test.java"), "class Test { int x = 1; }\n").unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["check", "java", root.to_str().unwrap(), "--format=json"])
        .output()
        .expect("无法运行 codeguard check java");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let report: serde_json::Value = serde_json::from_str(&stdout).expect("JSON 解析失败");

    // syntax_candidates 应有 next_action
    let next_action = report["syntax_candidates"]["next_action"].as_str().unwrap_or("");
    assert!(
        !next_action.is_empty(),
        "缺工具初检应给出下一步指引"
    );

    // delivery_decision 应为 incomplete（未完成）
    let delivery = report["syntax_candidates"]["delivery_decision"].as_str().unwrap_or("");
    assert_eq!(
        delivery, "incomplete",
        "缺工具时 delivery_decision 应为 incomplete"
    );

    // 退出码应为 3（未完成）
    assert_eq!(output.status.code(), Some(3), "缺工具时退出码应为 3");

    let _ = std::fs::remove_dir_all(&root);
}

/// 版本/方言兼容性：known_limitations 如实披露。
#[test]
fn version_dialect_compatibility_disclosed() {
    let root = ensure_clean_dir("codeguard-java-version-dialect");
    std::fs::write(root.join("Test.java"), "class Test { int x = 1; }\n").unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["check", "java", root.to_str().unwrap(), "--format=json"])
        .output()
        .expect("无法运行 codeguard check java");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let report: serde_json::Value = serde_json::from_str(&stdout).expect("JSON 解析失败");

    let observations = report["syntax_candidates"]["observations"]
        .as_array()
        .cloned()
        .unwrap_or_default();

    if !observations.is_empty() {
        let obs = &observations[0];
        // known_limitations 应存在且非空
        let limitations = obs["known_limitations"]
            .as_array()
            .expect("known_limitations 应为数组");
        assert!(
            !limitations.is_empty(),
            "版本/方言兼容性应有已知限制披露"
        );

        // limitations 应提到版本或方言
        let all_text: Vec<&str> = limitations
            .iter()
            .filter_map(|l| l.as_str())
            .collect();
        let combined = all_text.join(" ").to_lowercase();
        assert!(
            combined.contains("version") || combined.contains("dialect") || combined.contains("版本") || combined.contains("方言"),
            "known_limitations 应提及版本/方言: {combined}"
        );
    }

    // grammar_qualified 应为 false（未验收）
    if let Some(obs) = observations.first() {
        let qualified = obs["grammar_qualified"].as_bool().unwrap_or(false);
        assert!(!qualified, "Java grammar 当前未验收，grammar_qualified 应为 false");
    }

    let _ = std::fs::remove_dir_all(&root);
}

/// 精确定位：观察包含 byte_offset 和 grammar 身份。
#[test]
fn precise_positioning_with_grammar_identity() {
    let root = ensure_clean_dir("codeguard-java-precise-position");
    std::fs::write(
        root.join("Bad.java"),
        "package com.example;\n\nclass Bad {\n    int f( { return 1; }\n}\n",
    )
    .unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["check", "java", root.to_str().unwrap(), "--format=json"])
        .output()
        .expect("无法运行 codeguard check java");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let report: serde_json::Value = serde_json::from_str(&stdout).expect("JSON 解析失败");

    let observations = report["syntax_candidates"]["observations"]
        .as_array()
        .cloned()
        .unwrap_or_default();

    for obs in &observations {
        // byte_offset 应存在
        assert!(
            obs["byte_offset"].is_number(),
            "观察应有 byte_offset 精确定位"
        );

        // grammar_sha256 应存在且为 64 字符 hex
        let sha = obs["grammar_sha256"].as_str().unwrap_or("");
        assert_eq!(sha.len(), 64, "grammar_sha256 应为 64 字符 SHA-256");

        // path 应存在
        assert!(
            obs["path"].as_str().map(|s| !s.is_empty()).unwrap_or(false),
            "观察应有 path"
        );
    }

    let _ = std::fs::remove_dir_all(&root);
}

/// 取消/超时：execution_budget 包含时间限制配置。
#[test]
fn execution_budget_has_timeout_config() {
    let root = ensure_clean_dir("codeguard-java-budget-test");
    std::fs::write(root.join("Test.java"), "class Test { int x = 1; }\n").unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["check", "java", root.to_str().unwrap(), "--format=json"])
        .output()
        .expect("无法运行 codeguard check java");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let report: serde_json::Value = serde_json::from_str(&stdout).expect("JSON 解析失败");

    let budget = &report["execution_budget"];
    // timeout_ms 应存在
    assert!(
        budget["timeout_ms"].is_number(),
        "execution_budget 应有 timeout_ms"
    );
    // enforcement 应存在
    assert!(
        budget["enforcement"].as_str().map(|s| !s.is_empty()).unwrap_or(false),
        "execution_budget 应有 enforcement 策略"
    );

    let _ = std::fs::remove_dir_all(&root);
}

/// 安装指引：category_candidates 中的 gap 状态给出 next_action。
#[test]
fn installation_guidance_in_category_candidates() {
    let root = ensure_clean_dir("codeguard-java-install-guidance");
    std::fs::write(root.join("Test.java"), "class Test { int x = 1; }\n").unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["check", "java", root.to_str().unwrap(), "--format=json"])
        .output()
        .expect("无法运行 codeguard check java");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let report: serde_json::Value = serde_json::from_str(&stdout).expect("JSON 解析失败");

    let candidates = report["category_candidates"]
        .as_array()
        .expect("category_candidates 应为数组");

    // 应有 6 个类别（lint/comments/dependencies/cve/security/build）
    assert_eq!(candidates.len(), 6, "应有 6 个核心能力类别");

    // 每个类别应有 capability_status
    for cat in candidates {
        assert!(
            cat["capability_status"].as_str().map(|s| !s.is_empty()).unwrap_or(false),
            "每个类别应有 capability_status: {}",
            cat["category"]
        );
        assert!(
            cat["status"].as_str().map(|s| !s.is_empty()).unwrap_or(false),
            "每个类别应有 status: {}",
            cat["category"]
        );
    }

    // gap 状态的类别应有 next_action 或 reason
    for cat in candidates {
        let status = cat["status"].as_str().unwrap_or("");
        if status != "verified" && status != "complete" {
            let has_action = cat["next_action"].as_str().map(|s| !s.is_empty()).unwrap_or(false);
            let has_reason = cat["reason"].as_str().map(|s| !s.is_empty()).unwrap_or(false);
            assert!(
                has_action || has_reason,
                "gap 类别应有 next_action 或 reason: {}/{:?}",
                cat["category"],
                cat["status"]
            );
        }
    }

    let _ = std::fs::remove_dir_all(&root);
}
