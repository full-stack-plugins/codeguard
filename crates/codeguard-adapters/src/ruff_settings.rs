//! Ruff 0.16.8 原生逐文件设置的保守解析；设置观察不证明注释级 suppression 或批准覆盖。

use serde::Serialize;
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;

/// 本轮 Ruff `check --show-settings` 的有限规则观察。
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct RuffSettingsObservation {
    /// 原生设置输出原始字节 SHA-256，供私有日志比对。
    pub settings_sha256: String,
    /// 候选映射中被 Ruff 全局启用的规则；不推断其它规则。
    pub globally_enabled_mapped_rules: Vec<String>,
    /// 是否配置逐文件忽略；存在时精确文件覆盖仍需另行核验。
    pub per_file_ignores_present: bool,
    /// 设置本身不能证明源码内 `noqa` 等 suppression 不存在。
    pub coverage_proven: bool,
    /// 只供同轮原生报告交叉核对；不扩大公开规则映射或批准范围。
    #[serde(skip)]
    enabled_native_rules: BTreeSet<String>,
}

impl RuffSettingsObservation {
    /// 检查原生设置是否启用精确规则；参数为 Ruff 规则 ID，返回本轮设置观察结果。
    #[must_use]
    pub fn native_rule_enabled(&self, rule_id: &str) -> bool {
        self.enabled_native_rules.contains(rule_id)
    }
}

/// 仅接受固定版本实测的 `show-settings` 片段；其它格式必须保持未完成。
pub fn parse_ruff_settings(raw: &[u8]) -> Result<RuffSettingsObservation, &'static str> {
    if raw.is_empty() || raw.len() > 2 * 1024 * 1024 {
        return Err("ruff_settings_size_invalid");
    }
    let text = std::str::from_utf8(raw).map_err(|_| "ruff_settings_encoding_invalid")?;
    let mut lines = text.lines();
    let mut found_enabled = false;
    let mut enabled = BTreeSet::new();
    while let Some(line) = lines.next() {
        if line != "linter.rules.enabled = [" {
            continue;
        }
        if found_enabled {
            return Err("ruff_settings_duplicate_enabled_section");
        }
        found_enabled = true;
        let mut closed = false;
        for rule_line in lines.by_ref() {
            if rule_line == "]" {
                closed = true;
                break;
            }
            let rule_line = rule_line.trim();
            let (name, code) = rule_line
                .strip_suffix("),")
                .and_then(|line| line.rsplit_once(" ("))
                .ok_or("ruff_settings_rule_invalid")?;
            if name.is_empty()
                || code.is_empty()
                || code.len() > 12
                || !code
                    .bytes()
                    .all(|byte| byte.is_ascii_uppercase() || byte.is_ascii_digit())
                || !enabled.insert(code.to_owned())
            {
                return Err("ruff_settings_rule_invalid");
            }
        }
        if !closed {
            return Err("ruff_settings_enabled_unclosed");
        }
    }
    if !found_enabled {
        return Err("ruff_settings_enabled_missing");
    }
    let mut per_file = None;
    for line in text.lines() {
        if let Some(value) = line.strip_prefix("linter.per_file_ignores = ") {
            if per_file.is_some() || !matches!(value, "{}" | "{") {
                return Err("ruff_settings_per_file_invalid");
            }
            per_file = Some(value == "{");
        }
    }
    let per_file_ignores_present = per_file.ok_or("ruff_settings_per_file_missing")?;
    if per_file_ignores_present {
        let rest = text
            .split_once("linter.per_file_ignores = {\n")
            .map(|(_, rest)| rest)
            .ok_or("ruff_settings_per_file_invalid")?;
        let close = rest
            .find("\n}\n")
            .ok_or("ruff_settings_per_file_unclosed")?;
        if rest[..close]
            .lines()
            .any(|line| line.starts_with("linter."))
        {
            return Err("ruff_settings_per_file_unclosed");
        }
    }
    Ok(RuffSettingsObservation {
        settings_sha256: format!("{:x}", Sha256::digest(raw)),
        globally_enabled_mapped_rules: ["E501", "F401"]
            .into_iter()
            .filter(|rule| enabled.contains(*rule))
            .map(str::to_owned)
            .collect(),
        per_file_ignores_present,
        coverage_proven: false,
        enabled_native_rules: enabled,
    })
}
