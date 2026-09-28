//! Ruff 规则映射预览包的静态解析；映射存在不代表原生规则已启用或策略获批。

use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;

/// 一条已核对官方来源的 Ruff 原生规则映射。
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuffRuleMapping {
    /// 原生 Ruff 规则 ID。
    pub native_rule_id: String,
    /// CodeGuard 稳定规则 ID。
    pub codeguard_rule_id: String,
    /// 检查类别；本预览仅覆盖 lint。
    pub category: String,
    /// 官方规则说明地址。
    pub source_ref: String,
}

/// 随 Rust 二进制打包的版本化 Ruff 映射清单，批准状态始终独立核验。
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuffRulepack {
    /// 清单协议版本。
    pub schema_version: String,
    /// 稳定规则包 ID。
    pub id: String,
    /// 内容语义版本。
    pub version: String,
    /// 当前只允许未批准候选。
    pub status: String,
    /// 本清单只映射规则，不声明项目实际启用集合。
    pub scope: String,
    /// 官方规则目录地址。
    pub origin_url: String,
    /// 上游许可证标识。
    pub license: String,
    /// 上游许可证来源。
    pub license_ref: String,
    /// 已实际验收的原生工具版本。
    pub compatible_tool_versions: Vec<String>,
    /// 已验收的少量原生规则映射。
    pub mappings: Vec<RuffRuleMapping>,
    /// 原始清单字节摘要；不能作为批准来源。
    #[serde(skip)]
    pub sha256: String,
}

impl RuffRulepack {
    /// 查询精确原生规则；不根据前缀推断其它规则。
    #[must_use]
    pub fn mapping(&self, native_rule_id: &str) -> Option<&RuffRuleMapping> {
        self.mappings
            .iter()
            .find(|mapping| mapping.native_rule_id == native_rule_id)
    }

    /// 判断本预览包是否曾对该精确工具版本验收。
    #[must_use]
    pub fn supports_tool_version(&self, version: &str) -> bool {
        self.compatible_tool_versions
            .iter()
            .any(|supported| supported == version)
    }

    /// 预览包永不自授批准；正式批准须由受保护策略边界给出。
    #[must_use]
    pub fn approved(&self) -> bool {
        false
    }
}

/// 严格解析并计算原始字节身份；参数是固定上限内的清单字节。
pub fn parse_ruff_rulepack(raw: &[u8]) -> Result<RuffRulepack, &'static str> {
    if raw.is_empty() || raw.len() > 64 * 1024 {
        return Err("rulepack_size_invalid");
    }
    let mut pack: RuffRulepack =
        serde_json::from_slice(raw).map_err(|_| "rulepack_format_invalid")?;
    if pack.schema_version != "1.0.0"
        || pack.id != "python.ruff.lint.preview"
        || !valid_version(&pack.version)
        || pack.status != "candidate_unapproved"
        || pack.scope != "rule_mapping_only"
        || pack.origin_url != "https://docs.astral.sh/ruff/rules/"
        || pack.license != "MIT"
        || pack.license_ref != "https://github.com/astral-sh/ruff/blob/main/LICENSE"
        || pack.compatible_tool_versions != ["ruff 0.16.8"]
        || pack.mappings.len() != 2
    {
        return Err("rulepack_identity_invalid");
    }
    let mut seen = BTreeSet::new();
    for mapping in &pack.mappings {
        let expected_source = match mapping.native_rule_id.as_str() {
            "F401" => "https://docs.astral.sh/ruff/rules/unused-import/",
            "E501" => "https://docs.astral.sh/ruff/rules/line-too-long/",
            _ => return Err("rulepack_rule_unreviewed"),
        };
        if !seen.insert(mapping.native_rule_id.as_str())
            || mapping.codeguard_rule_id != format!("python.ruff.{}", mapping.native_rule_id)
            || mapping.category != "lint"
            || mapping.source_ref != expected_source
        {
            return Err("rulepack_mapping_invalid");
        }
    }
    if seen != BTreeSet::from(["E501", "F401"]) {
        return Err("rulepack_rules_missing");
    }
    pack.sha256 = format!("{:x}", Sha256::digest(raw));
    Ok(pack)
}

/// 读取随二进制编译的 Ruff 映射预览包；不读取项目可写规则文件。
pub fn bundled_ruff_rulepack() -> Result<RuffRulepack, &'static str> {
    parse_ruff_rulepack(include_bytes!(
        "../../../rulepacks/ruff_lint_preview_v1.json"
    ))
}

fn valid_version(version: &str) -> bool {
    let parts: Vec<_> = version.split('.').collect();
    parts.len() == 3
        && parts
            .into_iter()
            .all(|part| !part.is_empty() && part.bytes().all(|byte| byte.is_ascii_digit()))
}
