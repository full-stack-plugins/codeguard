//! 质量策略候选的严格语义解析；解析成功不赋予项目文件策略权威。

use codeguard_adapters::legacy_registry;
use codeguard_core::check_kind_belongs_to_category;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;

/// 候选文件中声明的一项必需检测义务模板。
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RequiredCheckCandidate {
    /// 策略内稳定身份。
    pub id: String,
    /// 规范语言 ID。
    pub language: String,
    /// 六类检查之一。
    pub category: String,
    /// 与类别对应的检测族。
    pub check_kind: String,
    /// 已固定的规则包字节摘要。
    pub rulepack_sha256: String,
    /// 必需的具体规则 ID。
    pub rule_ids: Vec<String>,
    /// 受检源码集合标识。
    pub source_sets: Vec<String>,
    /// 策略层阻断严重度集合；不从工具退出码猜测。
    pub blocking_severities: Vec<String>,
}

/// 候选策略对单个源码目标提出的排除声明；不等同于误报白名单。
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceExclusionCandidate {
    /// 排除项稳定身份。
    pub id: String,
    /// 仓库相对单文件路径，禁止通配和整目录。
    pub path: String,
    /// 被排除文件的固定字节身份；内容变化必须重审。
    pub file_sha256: String,
    /// 结构化原因代码。
    pub reason: String,
    /// UTC Unix 秒，受保护策略仍须验证期限与批准。
    pub expires_at: u64,
}

/// 经形状与内部一致性验证但尚未获批准的质量策略候选。
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QualityPolicyCandidate {
    /// 候选协议版本。
    pub schema_version: String,
    /// 质量策略身份。
    pub policy_id: String,
    /// 声称的修订号，不作为信任根。
    pub revision: String,
    /// 工具锁原始字节摘要。
    pub tool_lock_sha256: String,
    /// 必需检查义务模板。
    pub required_checks: Vec<RequiredCheckCandidate>,
    /// 候选的精确源码排除集合。
    pub source_exclusions: Vec<SourceExclusionCandidate>,
    /// 漏洞库允许的新鲜度时长。
    pub cve_freshness_hours: u32,
    /// 是否要求测试义务。
    pub tests_required: bool,
    /// 原始候选字节摘要，仅用于差异追踪。
    #[serde(skip)]
    pub sha256: String,
}

/// 校验候选 schema、必需集合、引用和精确排除；不验证外部批准。
pub fn parse_quality_policy_candidate(raw: &[u8]) -> Result<QualityPolicyCandidate, &'static str> {
    if raw.is_empty() || raw.len() > 128 * 1024 {
        return Err("quality_policy_candidate_size_invalid");
    }
    let mut policy: QualityPolicyCandidate =
        serde_json::from_slice(raw).map_err(|_| "quality_policy_candidate_format_invalid")?;
    if policy.schema_version != "1.0"
        || !safe_token(&policy.policy_id)
        || !safe_token(&policy.revision)
        || !digest(&policy.tool_lock_sha256)
        || policy.required_checks.is_empty()
        || !(1..=8_760).contains(&policy.cve_freshness_hours)
    {
        return Err("quality_policy_candidate_identity_invalid");
    }
    let mut check_ids = BTreeSet::new();
    let registry = legacy_registry().map_err(|_| "quality_policy_language_registry_invalid")?;
    for check in &policy.required_checks {
        if !check_ids.insert(check.id.as_str())
            || !safe_token(&check.id)
            || !safe_token(&check.language)
            || !registry
                .languages
                .iter()
                .any(|language| language.id == check.language)
            || !check_kind_belongs_to_category(&check.check_kind, &check.category)
            || !digest(&check.rulepack_sha256)
            || !unique_tokens(&check.rule_ids)
            || !unique_tokens(&check.source_sets)
            || !unique_severities(&check.blocking_severities)
        {
            return Err("quality_policy_candidate_required_check_invalid");
        }
    }
    let mut exclusion_ids = BTreeSet::new();
    let mut exclusion_paths = BTreeSet::new();
    for exclusion in &policy.source_exclusions {
        if !exclusion_ids.insert(exclusion.id.as_str())
            || !exclusion_paths.insert(exclusion.path.as_str())
            || !safe_token(&exclusion.id)
            || !safe_relative_source_path(&exclusion.path)
            || !digest(&exclusion.file_sha256)
            || !matches!(
                exclusion.reason.as_str(),
                "generated_source" | "vendored_source" | "non_applicable_source"
            )
            || exclusion.expires_at == 0
        {
            return Err("quality_policy_candidate_exclusion_invalid");
        }
    }
    policy.sha256 = format!("{:x}", Sha256::digest(raw));
    Ok(policy)
}

fn unique_tokens(values: &[String]) -> bool {
    !values.is_empty() && values.iter().all(|value| safe_token(value)) && {
        let mut seen = BTreeSet::new();
        values.iter().all(|value| seen.insert(value.as_str()))
    }
}

fn unique_severities(values: &[String]) -> bool {
    !values.is_empty()
        && values.iter().all(|value| {
            matches!(
                value.as_str(),
                "error" | "warning" | "critical" | "high" | "medium" | "low"
            )
        })
        && {
            let mut seen = BTreeSet::new();
            values.iter().all(|value| seen.insert(value.as_str()))
        }
}

fn safe_token(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'-' | b'_' | b':'))
}

fn safe_relative_source_path(path: &str) -> bool {
    !path.is_empty()
        && path.len() <= 4096
        && !path.contains('\\')
        && !path.contains(':')
        && path.split('/').all(|part| {
            !matches!(part, "" | "." | "..")
                && part
                    .chars()
                    .all(|ch| !ch.is_control() && ch != '*' && ch != '?' && ch != '[' && ch != ']')
        })
}

fn digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}
