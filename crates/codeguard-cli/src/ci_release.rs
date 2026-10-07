//! CI 集成与发行验证模块（11.8/11.9）。
//!
//! 实现 CI 集成、发行验证：版本摘要闭环、插件 lock、市场同步。

use serde_json::{Value, json};

/// CI 集成结果。
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum CiResult {
    /// 成功。
    Success,
    /// 测试失败。
    TestFailed,
    /// 构建失败。
    BuildFailed,
    /// 发行验证失败。
    ReleaseFailed,
}

impl CiResult {
    pub(crate) fn as_str(&self) -> &'static str {
        match self {
            Self::Success => "success",
            Self::TestFailed => "test_failed",
            Self::BuildFailed => "build_failed",
            Self::ReleaseFailed => "release_failed",
        }
    }
}

/// 发行验证结果。
pub(crate) struct ReleaseVerification {
    pub version_match: bool,
    pub digest_match: bool,
    pub plugin_lock_valid: bool,
    pub market_synced: bool,
}

/// 执行 CI 集成。
pub(crate) fn ci_integration(
    tests_passed: bool,
    build_succeeded: bool,
    release_verified: bool,
) -> CiResult {
    if !build_succeeded {
        CiResult::BuildFailed
    } else if !tests_passed {
        CiResult::TestFailed
    } else if !release_verified {
        CiResult::ReleaseFailed
    } else {
        CiResult::Success
    }
}

/// 验证发行。
pub(crate) fn verify_release(
    expected_version: &str,
    actual_version: &str,
    expected_digest: &str,
    actual_digest: &str,
    plugin_lock_valid: bool,
    market_synced: bool,
) -> ReleaseVerification {
    ReleaseVerification {
        version_match: expected_version == actual_version,
        digest_match: expected_digest == actual_digest,
        plugin_lock_valid,
        market_synced,
    }
}

/// 生成 CI 发行报告。
pub(crate) fn ci_release_report(ci_result: &CiResult, release: &ReleaseVerification) -> Value {
    json!({
        "schema_version": "0.1.0",
        "report_type": "ci_release",
        "ci_result": ci_result.as_str(),
        "version_match": release.version_match,
        "digest_match": release.digest_match,
        "plugin_lock_valid": release.plugin_lock_valid,
        "market_synced": release.market_synced,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ci_success() {
        let result = ci_integration(true, true, true);
        assert_eq!(result, CiResult::Success);
    }

    #[test]
    fn ci_test_failed() {
        let result = ci_integration(false, true, true);
        assert_eq!(result, CiResult::TestFailed);
    }

    #[test]
    fn ci_build_failed() {
        let result = ci_integration(true, false, true);
        assert_eq!(result, CiResult::BuildFailed);
    }

    #[test]
    fn verify_release_all_match() {
        let release = verify_release("1.0.0", "1.0.0", "abc", "abc", true, true);
        assert!(release.version_match);
        assert!(release.digest_match);
    }

    #[test]
    fn verify_release_version_mismatch() {
        let release = verify_release("1.0.0", "2.0.0", "abc", "abc", true, true);
        assert!(!release.version_match);
    }

    #[test]
    fn ci_release_report_contains_status() {
        let ci_result = ci_integration(true, true, true);
        let release = verify_release("1.0.0", "1.0.0", "abc", "abc", true, true);
        let report = ci_release_report(&ci_result, &release);
        assert_eq!(report["ci_result"], "success");
    }
}
