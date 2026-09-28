//! pip-audit 2.x JSON 的保守观察；仅接受禁用修复、描述并启用别名的原生输出形状。

use crate::{PipAuditDependency, PipAuditFinding, PipAuditObservation, parse_unique_json};
use serde_json::Value;
use std::collections::BTreeSet;

/// 解析受控 pip-audit 的 JSON 和退出状态；输入版本必须由执行器独立观察。
/// 参数为有界 stdout、预期/实测工具版本及原生退出码；返回值不授予漏洞库覆盖权威。
pub fn parse_pip_audit_json(
    bytes: &[u8],
    expected_version: &str,
    observed_version: &str,
    exit: Option<i32>,
) -> Result<PipAuditObservation, &'static str> {
    let invalid = "pip_audit_report_invalid";
    if expected_version != observed_version
        || semver::Version::parse(expected_version)
            .ok()
            .is_none_or(|version| version.major != 2 || !version.pre.is_empty())
    {
        return Err("pip_audit_version_unverified");
    }
    let report = parse_unique_json(bytes).map_err(|_| invalid)?;
    if !exact(&report, &["dependencies", "fixes"]) {
        return Err(invalid);
    }
    let fixes = report["fixes"].as_array().ok_or(invalid)?;
    if !fixes.is_empty() {
        return Err("pip_audit_mutating_result_unexpected");
    }
    let items = report["dependencies"]
        .as_array()
        .filter(|items| items.len() <= 10_000)
        .ok_or(invalid)?;
    let mut dependencies = Vec::with_capacity(items.len());
    let mut findings = Vec::new();
    let mut names = BTreeSet::new();
    let mut identities = BTreeSet::new();
    for item in items {
        if exact(item, &["name", "skip_reason"]) {
            return Err("pip_audit_dependency_skipped");
        }
        if !exact(item, &["name", "version", "vulns"]) {
            return Err(invalid);
        }
        let name = bounded_text(&item["name"], 214).ok_or(invalid)?;
        let version = bounded_text(&item["version"], 128).ok_or(invalid)?;
        if !package_name(name) || !version_text(version) || !names.insert(canonical_name(name)) {
            return Err(invalid);
        }
        let vulns = item["vulns"]
            .as_array()
            .filter(|items| items.len() <= 10_000)
            .ok_or(invalid)?;
        for vuln in vulns {
            if !exact(vuln, &["id", "fix_versions", "aliases"]) {
                return Err(invalid);
            }
            let advisory_id = bounded_text(&vuln["id"], 256).ok_or(invalid)?;
            if !identifier(advisory_id)
                || !identities.insert((name.to_owned(), version.to_owned(), advisory_id.to_owned()))
            {
                return Err(invalid);
            }
            let fix_versions = strings(&vuln["fix_versions"], 128, version_text)?;
            let aliases = strings(&vuln["aliases"], 256, identifier)?;
            let cve_aliases = aliases
                .iter()
                .filter(|alias| is_cve(alias))
                .cloned()
                .collect();
            findings.push(PipAuditFinding {
                advisory_id: advisory_id.into(),
                package_name: name.into(),
                package_version: version.into(),
                aliases,
                cve_aliases,
                fix_versions,
            });
        }
        dependencies.push(PipAuditDependency {
            name: name.into(),
            version: version.into(),
        });
    }
    if exit != Some(if findings.is_empty() { 0 } else { 1 }) {
        return Err("pip_audit_execution_incomplete");
    }
    Ok(PipAuditObservation {
        advisory_coverage: "not_evaluated",
        dependencies,
        findings,
    })
}

fn exact(value: &Value, keys: &[&str]) -> bool {
    value.as_object().is_some_and(|object| {
        object.len() == keys.len() && keys.iter().all(|key| object.contains_key(*key))
    })
}

fn bounded_text(value: &Value, max: usize) -> Option<&str> {
    value.as_str().filter(|text| {
        !text.is_empty()
            && text.len() <= max
            && !text.chars().any(char::is_control)
            && text.trim() == *text
    })
}

fn package_name(name: &str) -> bool {
    name.bytes()
        .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
        && name
            .bytes()
            .next()
            .is_some_and(|byte| byte.is_ascii_alphanumeric())
        && name
            .bytes()
            .last()
            .is_some_and(|byte| byte.is_ascii_alphanumeric())
}

fn canonical_name(name: &str) -> String {
    let mut canonical = String::new();
    let mut separator = false;
    for byte in name.bytes() {
        if matches!(byte, b'-' | b'_' | b'.') {
            if !separator {
                canonical.push('-');
            }
            separator = true;
        } else {
            canonical.push(char::from(byte.to_ascii_lowercase()));
            separator = false;
        }
    }
    canonical
}

fn version_text(version: &str) -> bool {
    version.bytes().all(|byte| {
        byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'-' | b'_' | b'+' | b'!')
    })
}

fn identifier(value: &str) -> bool {
    value
        .bytes()
        .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b':'))
}

fn is_cve(alias: &str) -> bool {
    let Some((year, serial)) = alias
        .strip_prefix("CVE-")
        .and_then(|rest| rest.split_once('-'))
    else {
        return false;
    };
    year.len() == 4
        && year.bytes().all(|byte| byte.is_ascii_digit())
        && serial.len() >= 4
        && serial.bytes().all(|byte| byte.is_ascii_digit())
}

fn strings(
    value: &Value,
    max: usize,
    valid: fn(&str) -> bool,
) -> Result<Vec<String>, &'static str> {
    let invalid = "pip_audit_report_invalid";
    let items = value
        .as_array()
        .filter(|items| items.len() <= 10_000)
        .ok_or(invalid)?;
    let mut seen = BTreeSet::new();
    items
        .iter()
        .map(|item| {
            let text = bounded_text(item, max).ok_or(invalid)?;
            if !valid(text) || !seen.insert(text) {
                return Err(invalid);
            }
            Ok(text.into())
        })
        .collect()
}
