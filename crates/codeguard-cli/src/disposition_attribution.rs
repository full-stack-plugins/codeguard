//! 处置归因模块。
//!
//! 将 findings 归因为 code/dependency/environment/target/policy 五类。
//! policy_resolved 不计代码修复；例外保留未解决事实和期限。

use serde_json::{Value, json};

/// 处置类别。
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Disposition {
    /// 代码问题（需修复源码）。
    Code,
    /// 依赖问题（需升级/更换依赖）。
    Dependency,
    /// 环境问题（需安装工具/恢复配置）。
    Environment,
    /// 目标问题（需调整构建目标/平台）。
    Target,
    /// 策略问题（需调整规则/策略）。
    Policy,
}

impl Disposition {
    /// 从字符串解析处置类别。
    pub(crate) fn from_str(s: &str) -> Option<Self> {
        match s {
            "code" => Some(Self::Code),
            "dependency" => Some(Self::Dependency),
            "environment" => Some(Self::Environment),
            "target" => Some(Self::Target),
            "policy" => Some(Self::Policy),
            _ => None,
        }
    }

    /// 转换为字符串。
    pub(crate) fn as_str(&self) -> &'static str {
        match self {
            Self::Code => "code",
            Self::Dependency => "dependency",
            Self::Environment => "environment",
            Self::Target => "target",
            Self::Policy => "policy",
        }
    }

    /// 是否计入代码修复。
    pub(crate) fn counts_as_code_fix(&self) -> bool {
        matches!(self, Self::Code)
    }
}

/// 处置归因结果。
pub(crate) struct Attribution {
    pub disposition: Disposition,
    pub reason: String,
    pub is_exception: bool,
    pub exception_expiry: Option<String>,
}

/// 归因 finding 到处置类别。
pub(crate) fn attribute_finding(finding: &Value) -> Attribution {
    let reason_code = finding["reason_code"].as_str().unwrap_or("");
    let checker_id = finding["checker_id"].as_str().unwrap_or("");
    
    // 环境问题：工具缺失、配置缺失
    if reason_code.contains("tool_not_found")
        || reason_code.contains("config_not_found")
        || reason_code.contains("environment")
        || reason_code.contains("native_tool_not_available")
    {
        return Attribution {
            disposition: Disposition::Environment,
            reason: reason_code.to_string(),
            is_exception: false,
            exception_expiry: None,
        };
    }
    
    // 依赖问题：CVE、漏洞、依赖缺失
    if reason_code.contains("cve") || reason_code.contains("vulnerability") || reason_code.contains("dependency") {
        return Attribution {
            disposition: Disposition::Dependency,
            reason: reason_code.to_string(),
            is_exception: false,
            exception_expiry: None,
        };
    }
    
    // 策略问题：规则禁用、策略调整
    if reason_code.contains("policy") || reason_code.contains("rule_disabled") || reason_code.contains("suppressed") {
        return Attribution {
            disposition: Disposition::Policy,
            reason: reason_code.to_string(),
            is_exception: false,
            exception_expiry: None,
        };
    }
    
    // 目标问题：构建目标、平台
    if reason_code.contains("target") || reason_code.contains("platform") {
        return Attribution {
            disposition: Disposition::Target,
            reason: reason_code.to_string(),
            is_exception: false,
            exception_expiry: None,
        };
    }
    
    // 默认：代码问题
    Attribution {
        disposition: Disposition::Code,
        reason: reason_code.to_string(),
        is_exception: false,
        exception_expiry: None,
    }
}

/// 生成处置归因报告。
pub(crate) fn attribution_report(findings: &[Value]) -> Value {
    let mut code_count = 0;
    let mut dependency_count = 0;
    let mut environment_count = 0;
    let mut target_count = 0;
    let mut policy_count = 0;
    
    for finding in findings {
        let attr = attribute_finding(finding);
        match attr.disposition {
            Disposition::Code => code_count += 1,
            Disposition::Dependency => dependency_count += 1,
            Disposition::Environment => environment_count += 1,
            Disposition::Target => target_count += 1,
            Disposition::Policy => policy_count += 1,
        }
    }
    
    json!({
        "schema_version": "0.1.0",
        "report_type": "disposition_attribution",
        "code": code_count,
        "dependency": dependency_count,
        "environment": environment_count,
        "target": target_count,
        "policy": policy_count,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn code_finding_attributed_to_code() {
        let finding = json!({"reason_code": "unused_variable", "checker_id": "ruff"});
        let attr = attribute_finding(&finding);
        assert_eq!(attr.disposition, Disposition::Code);
        assert!(attr.disposition.counts_as_code_fix());
    }

    #[test]
    fn environment_finding_attributed_to_environment() {
        let finding = json!({"reason_code": "native_tool_not_available", "checker_id": "phpcs"});
        let attr = attribute_finding(&finding);
        assert_eq!(attr.disposition, Disposition::Environment);
        assert!(!attr.disposition.counts_as_code_fix());
    }

    #[test]
    fn cve_finding_attributed_to_dependency() {
        let finding = json!({"reason_code": "cve_vulnerability_found", "checker_id": "composer_audit"});
        let attr = attribute_finding(&finding);
        assert_eq!(attr.disposition, Disposition::Dependency);
    }

    #[test]
    fn policy_finding_attributed_to_policy() {
        let finding = json!({"reason_code": "rule_disabled", "checker_id": "credo"});
        let attr = attribute_finding(&finding);
        assert_eq!(attr.disposition, Disposition::Policy);
    }

    #[test]
    fn policy_resolved_not_counted_as_code_fix() {
        let finding = json!({"reason_code": "policy_resolved", "checker_id": "credo"});
        let attr = attribute_finding(&finding);
        assert_eq!(attr.disposition, Disposition::Policy);
        assert!(!attr.disposition.counts_as_code_fix());
    }

    #[test]
    fn attribution_report_counts_all_categories() {
        let findings = vec![
            json!({"reason_code": "unused_variable", "checker_id": "ruff"}),
            json!({"reason_code": "native_tool_not_available", "checker_id": "phpcs"}),
            json!({"reason_code": "cve_vulnerability_found", "checker_id": "composer_audit"}),
        ];
        let report = attribution_report(&findings);
        assert_eq!(report["code"], 1);
        assert_eq!(report["environment"], 1);
        assert_eq!(report["dependency"], 1);
    }
}
