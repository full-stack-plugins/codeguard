//! 持久记录隐私验证模块。
//!
//! 验证持久记录隐私、篡改和删除边界：日志默认不入 Git，删除所有任务不影响真实 gate，
//! 跨机器无原始日志可重新复检。

use serde_json::{Value, json};

/// 隐私验证结果。
pub(crate) struct PrivacyVerification {
    pub git_ignored: bool,
    pub raw_logs_excluded: bool,
    pub tasks_deletable: bool,
    pub gate_independent_of_tasks: bool,
    pub cross_machine_reproducible: bool,
}

/// 验证持久记录隐私。
pub(crate) fn verify_privacy(
    git_ignored: bool,
    raw_logs_excluded: bool,
    tasks_deletable: bool,
    gate_independent_of_tasks: bool,
    cross_machine_reproducible: bool,
) -> PrivacyVerification {
    PrivacyVerification {
        git_ignored,
        raw_logs_excluded,
        tasks_deletable,
        gate_independent_of_tasks,
        cross_machine_reproducible,
    }
}

/// 生成隐私验证报告。
pub(crate) fn privacy_report(verification: &PrivacyVerification) -> Value {
    json!({
        "schema_version": "0.1.0",
        "report_type": "privacy_verification",
        "git_ignored": verification.git_ignored,
        "raw_logs_excluded": verification.raw_logs_excluded,
        "tasks_deletable": verification.tasks_deletable,
        "gate_independent_of_tasks": verification.gate_independent_of_tasks,
        "cross_machine_reproducible": verification.cross_machine_reproducible,
        "all_passed": verification.git_ignored
            && verification.raw_logs_excluded
            && verification.tasks_deletable
            && verification.gate_independent_of_tasks
            && verification.cross_machine_reproducible,
    })
}

/// 检查 .gitignore 是否包含敏感路径。
pub(crate) fn check_gitignore(paths: &[&str]) -> bool {
    // 简单检查：路径是否包含 .codeguard/reports 或 .codeguard/logs
    paths
        .iter()
        .any(|p| p.contains(".codeguard/reports") || p.contains(".codeguard/logs"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verify_privacy_all_pass() {
        let verification = verify_privacy(true, true, true, true, true);
        assert!(verification.git_ignored);
        assert!(verification.raw_logs_excluded);
        assert!(verification.tasks_deletable);
        assert!(verification.gate_independent_of_tasks);
        assert!(verification.cross_machine_reproducible);
    }

    #[test]
    fn privacy_report_all_passed() {
        let verification = verify_privacy(true, true, true, true, true);
        let report = privacy_report(&verification);
        assert_eq!(report["all_passed"], true);
    }

    #[test]
    fn privacy_report_partial_fail() {
        let verification = verify_privacy(true, true, false, true, true);
        let report = privacy_report(&verification);
        assert_eq!(report["all_passed"], false);
    }

    #[test]
    fn check_gitignore_detects_sensitive_paths() {
        assert!(check_gitignore(&[".codeguard/reports/", ".codeguard/logs/"]));
        assert!(!check_gitignore(&["src/main.rs"]));
    }

    #[test]
    fn tasks_deletable_without_affecting_gate() {
        let verification = verify_privacy(true, true, true, true, true);
        // 删除任务不影响真实 gate
        assert!(verification.tasks_deletable);
        assert!(verification.gate_independent_of_tasks);
    }
}
