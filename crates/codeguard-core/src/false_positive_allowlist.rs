//! 误报白名单的精确身份比较；此模块不验证批准，也不改变门禁。

use crate::CHECK_CATEGORIES;
use serde::{Deserialize, Serialize};

/// 可被精确裁定的源码或依赖目标；整目录与通配规则不在本契约内。
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
#[serde(deny_unknown_fields)]
pub enum AllowlistTarget {
    /// 本次实际检查的项目内源码文件及字节摘要。
    Source { path: String, file_sha256: String },
    /// 解析后的依赖节点、图及原生漏洞公告身份。
    Dependency {
        component: String,
        version: String,
        graph_sha256: String,
        advisory_id: String,
    },
}

/// 单条原生 finding 的完整匹配身份；批准来源与期限由策略边界另行验证。
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FalsePositiveIdentity {
    /// 稳定发现 ID。
    pub finding_id: String,
    /// 原生检查器 ID。
    pub checker_id: String,
    /// 原生规则 ID。
    pub native_rule_id: String,
    /// 检查类别。
    pub category: String,
    /// 源码或依赖的精确目标。
    pub target: AllowlistTarget,
    /// 原生证据、结构化符号和目标组合所得的发现指纹。
    pub finding_fingerprint: String,
    /// 本次原生工具制品摘要。
    pub tool_sha256: String,
    /// 本次 Rust 适配器摘要。
    pub adapter_sha256: String,
    /// 本次规则包摘要。
    pub rulepack_sha256: String,
}

/// 比较失败原因；缺少身份不能按匹配成功处理。
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum IdentityMismatch {
    /// 本次观察不具有可核查的精确身份。
    InvalidObservation,
    /// 候选裁定缺字段或含宽泛范围。
    InvalidDecision,
    /// 两个有效身份对应不同的问题。
    DifferentIdentity,
}

/// 比较本次原生发现与候选裁定的完整身份；返回成功**不代表**已获可信批准或可交付。
///
/// `observed` 必须来自本次有效原生报告及内容复核，`decision` 必须再由独立策略边界
/// 验证批准身份、有限期限和可信政策修订。调用方不能仅凭此函数放行。
pub fn match_false_positive_identity(
    observed: &FalsePositiveIdentity,
    decision: &FalsePositiveIdentity,
) -> Result<(), IdentityMismatch> {
    if !valid_identity(observed) {
        return Err(IdentityMismatch::InvalidObservation);
    }
    if !valid_identity(decision) {
        return Err(IdentityMismatch::InvalidDecision);
    }
    if observed != decision {
        return Err(IdentityMismatch::DifferentIdentity);
    }
    Ok(())
}

/// 校验单个候选身份是否具备完整精确范围；此校验不包含来源授权或有效期。
#[must_use]
pub fn valid_false_positive_identity(identity: &FalsePositiveIdentity) -> bool {
    valid_identity(identity)
}

fn valid_identity(identity: &FalsePositiveIdentity) -> bool {
    [
        identity.finding_id.as_str(),
        identity.checker_id.as_str(),
        identity.native_rule_id.as_str(),
        identity.category.as_str(),
    ]
    .into_iter()
    .all(exact_token)
        && CHECK_CATEGORIES.contains(&identity.category.as_str())
        && [
            identity.finding_fingerprint.as_str(),
            identity.tool_sha256.as_str(),
            identity.adapter_sha256.as_str(),
            identity.rulepack_sha256.as_str(),
        ]
        .into_iter()
        .all(valid_sha256)
        && match &identity.target {
            AllowlistTarget::Source { path, file_sha256 } => {
                safe_relative_path(path) && valid_sha256(file_sha256)
            }
            AllowlistTarget::Dependency {
                component,
                version,
                graph_sha256,
                advisory_id,
            } => {
                exact_token(component)
                    && exact_token(version)
                    && valid_sha256(graph_sha256)
                    && exact_token(advisory_id)
            }
        }
}

fn exact_token(value: &str) -> bool {
    !value.is_empty()
        && !value
            .chars()
            .any(|c| c.is_control() || c == '*' || c == '?')
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn safe_relative_path(path: &str) -> bool {
    !path.starts_with('/')
        && !path.contains('\\')
        && !path.contains(':')
        && path
            .split('/')
            .all(|part| !matches!(part, "" | "." | "..") && exact_token(part))
}
