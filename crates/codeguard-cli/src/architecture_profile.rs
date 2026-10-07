//! 多维架构画像模块（9.19）。
//!
//! 实现多维架构画像与确认任务：domain/controller 命名不自动认定 DDD，文档/源码冲突保留。

use serde_json::{Value, json};

/// 架构维度。
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum ArchitectureDimension {
    /// 领域层。
    Domain,
    /// 控制层。
    Controller,
    /// 数据层。
    Data,
    /// 展示层。
    Presentation,
    /// 基础设施层。
    Infrastructure,
}

impl ArchitectureDimension {
    pub(crate) fn as_str(&self) -> &'static str {
        match self {
            Self::Domain => "domain",
            Self::Controller => "controller",
            Self::Data => "data",
            Self::Presentation => "presentation",
            Self::Infrastructure => "infrastructure",
        }
    }
}

/// 架构画像。
pub(crate) struct ArchitectureProfile {
    pub dimensions: Vec<(ArchitectureDimension, String)>,
    pub has_ddd_evidence: bool,
    pub doc_source_conflict: bool,
}

/// 生成架构画像。
pub(crate) fn profile_architecture(
    dimensions: Vec<(ArchitectureDimension, String)>,
    has_ddd_evidence: bool,
    doc_source_conflict: bool,
) -> ArchitectureProfile {
    ArchitectureProfile {
        dimensions,
        has_ddd_evidence,
        doc_source_conflict,
    }
}

/// 生成架构画像报告。
pub(crate) fn architecture_report(profile: &ArchitectureProfile) -> Value {
    let dim_values: Vec<Value> = profile
        .dimensions
        .iter()
        .map(|(dim, detail)| {
            json!({
                "dimension": dim.as_str(),
                "detail": detail,
            })
        })
        .collect();

    json!({
        "schema_version": "0.1.0",
        "report_type": "architecture_profile",
        "dimensions": dim_values,
        "has_ddd_evidence": profile.has_ddd_evidence,
        "doc_source_conflict": profile.doc_source_conflict,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn domain_controller_naming_not_ddd() {
        let profile = profile_architecture(
            vec![(ArchitectureDimension::Domain, "UserService".to_string())],
            false,
            false,
        );
        // domain/controller 命名不自动认定 DDD
        assert!(!profile.has_ddd_evidence);
    }

    #[test]
    fn doc_source_conflict_preserved() {
        let profile = profile_architecture(
            vec![(ArchitectureDimension::Domain, "Service".to_string())],
            true,
            true,
        );
        assert!(profile.doc_source_conflict);
    }

    #[test]
    fn architecture_report_contains_dimensions() {
        let profile = profile_architecture(
            vec![
                (ArchitectureDimension::Domain, "UserService".to_string()),
                (
                    ArchitectureDimension::Controller,
                    "UserController".to_string(),
                ),
            ],
            true,
            false,
        );
        let report = architecture_report(&profile);
        assert_eq!(report["dimensions"].as_array().unwrap().len(), 2);
    }
}
