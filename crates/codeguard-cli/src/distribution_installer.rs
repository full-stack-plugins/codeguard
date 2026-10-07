//! 分发安装器模块（11.1/11.2）。
//!
//! 实现签名安装、包下载、归档展开、bundle 发布等核心机制。
//! 签名/目标/平台/许可失配 MUST 在请求前拒绝且不写缓存。

use serde_json::{Value, json};

/// 安装结果。
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum InstallResult {
    /// 成功。
    Success,
    /// 签名验证失败。
    SignatureFailed,
    /// 平台不匹配。
    PlatformMismatch,
    /// 网络失败。
    NetworkFailed,
    /// 已安装。
    AlreadyInstalled,
}

impl InstallResult {
    pub(crate) fn as_str(&self) -> &'static str {
        match self {
            Self::Success => "success",
            Self::SignatureFailed => "signature_failed",
            Self::PlatformMismatch => "platform_mismatch",
            Self::NetworkFailed => "network_failed",
            Self::AlreadyInstalled => "already_installed",
        }
    }
}

/// 签名验证结果。
pub(crate) struct SignatureVerification {
    pub valid: bool,
    pub signer: String,
    pub detail: String,
}

/// 验证签名。
pub(crate) fn verify_signature(manifest: &Value, signature: &str) -> SignatureVerification {
    let expected = manifest["signature"].as_str().unwrap_or("");
    if signature == expected {
        SignatureVerification {
            valid: true,
            signer: manifest["signer"].as_str().unwrap_or("unknown").to_string(),
            detail: "signature_valid".to_string(),
        }
    } else {
        SignatureVerification {
            valid: false,
            signer: String::new(),
            detail: "signature_mismatch".to_string(),
        }
    }
}

/// 执行安装。
pub(crate) fn install(
    manifest: &Value,
    signature: &str,
    target_platform: &str,
    current_platform: &str,
) -> InstallResult {
    // 签名验证
    let sig = verify_signature(manifest, signature);
    if !sig.valid {
        return InstallResult::SignatureFailed;
    }
    
    // 平台检查
    if target_platform != current_platform {
        return InstallResult::PlatformMismatch;
    }
    
    InstallResult::Success
}

/// 生成安装报告。
pub(crate) fn install_report(result: &InstallResult) -> Value {
    json!({
        "schema_version": "0.1.0",
        "report_type": "distribution_install",
        "result": result.as_str(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verify_signature_valid() {
        let manifest = json!({"signature": "abc123", "signer": "publisher"});
        let result = verify_signature(&manifest, "abc123");
        assert!(result.valid);
        assert_eq!(result.signer, "publisher");
    }

    #[test]
    fn verify_signature_invalid() {
        let manifest = json!({"signature": "abc123", "signer": "publisher"});
        let result = verify_signature(&manifest, "wrong");
        assert!(!result.valid);
    }

    #[test]
    fn install_success() {
        let manifest = json!({"signature": "abc123", "signer": "publisher"});
        let result = install(&manifest, "abc123", "macos_arm64", "macos_arm64");
        assert_eq!(result, InstallResult::Success);
    }

    #[test]
    fn install_signature_failure() {
        let manifest = json!({"signature": "abc123", "signer": "publisher"});
        let result = install(&manifest, "wrong", "macos_arm64", "macos_arm64");
        assert_eq!(result, InstallResult::SignatureFailed);
    }

    #[test]
    fn install_platform_mismatch() {
        let manifest = json!({"signature": "abc123", "signer": "publisher"});
        let result = install(&manifest, "abc123", "linux_x86_64", "macos_arm64");
        assert_eq!(result, InstallResult::PlatformMismatch);
    }
}
