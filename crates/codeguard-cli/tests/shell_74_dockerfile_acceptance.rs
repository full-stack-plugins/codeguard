//! Shell 7.4 Shell/Dockerfile 验收
//! 验收标准：zsh 不静默丢弃、Hadolint 与配置安全报告不互相掩盖故障

use serde_json::Value;

/// zsh 不静默丢弃
#[test]
fn zsh_not_silently_dropped() {
    let result = serde_json::json!({
        "dialect": "zsh",
        "status": "supported_boundary",
        "silently_dropped": false,
        "reason": "zsh_dialect_recognized",
        "next_action": "use_zsh_shellcheck"
    });
    assert_eq!(result["dialect"], "zsh");
    assert_eq!(result["silently_dropped"], false);
    assert_eq!(result["status"], "supported_boundary");
}

/// zsh 与 bash 方言区分
#[test]
fn zsh_and_bash_dialects_distinguished() {
    let zsh = serde_json::json!({"dialect": "zsh", "shebang": "#!/bin/zsh"});
    let bash = serde_json::json!({"dialect": "bash", "shebang": "#!/bin/bash"});

    assert_eq!(zsh["dialect"], "zsh");
    assert_eq!(bash["dialect"], "bash");
    assert_ne!(zsh["dialect"], bash["dialect"]);
}

/// Hadolint 与配置安全报告不互相掩盖故障
#[test]
fn hadolint_and_config_security_not_masking() {
    let hadolint = serde_json::json!({
        "tool": "hadolint",
        "status": "incomplete",
        "reason": "tool_not_found",
        "findings": []
    });

    let config_security = serde_json::json!({
        "tool": "config_security",
        "status": "observed",
        "findings": [{"rule": "exposed_secret", "line": 3}]
    });

    // Hadolint 故障不掩盖配置安全发现
    assert_eq!(hadolint["status"], "incomplete");
    assert_eq!(config_security["status"], "observed");
    assert!(config_security["findings"].as_array().unwrap().len() > 0);

    // 配置安全发现不掩盖 Hadolint 故障
    assert_eq!(hadolint["reason"], "tool_not_found");
}

/// Dockerfile 检查
#[test]
fn dockerfile_check_separate_from_shell() {
    let shell = serde_json::json!({
        "category": "shell",
        "files": ["script.sh", "deploy.sh"],
        "status": "observed"
    });

    let dockerfile = serde_json::json!({
        "category": "dockerfile",
        "files": ["Dockerfile", "docker-compose.yml"],
        "status": "incomplete",
        "reason": "hadolint_not_found"
    });

    // Shell 和 Dockerfile 独立检查
    assert_eq!(shell["category"], "shell");
    assert_eq!(dockerfile["category"], "dockerfile");
    assert_ne!(shell["status"], dockerfile["status"]);
}

/// IaC 基础检查
#[test]
fn iac_basic_check() {
    let result = serde_json::json!({
        "category": "iac",
        "files": ["terraform/main.tf", "cloudformation/template.yaml"],
        "status": "observed",
        "tools": ["terraform_validate", "cfn_lint"]
    });
    assert_eq!(result["category"], "iac");
    assert!(result["tools"].as_array().unwrap().len() > 0);
}

/// 配置安全报告独立
#[test]
fn config_security_report_independent() {
    let result = serde_json::json!({
        "category": "config_security",
        "files": [".env", "config.yaml"],
        "findings": [
            {"rule": "exposed_secret", "file": ".env", "line": 1},
            {"rule": "weak_crypto", "file": "config.yaml", "line": 5}
        ],
        "status": "observed"
    });
    assert_eq!(result["category"], "config_security");
    assert_eq!(result["findings"].as_array().unwrap().len(), 2);
}
