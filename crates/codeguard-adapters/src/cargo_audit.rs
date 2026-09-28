//! cargo-audit JSON 的保守观察；只解释原生结果，不授予数据库时效或安全通过。

use crate::parse_unique_json;
use serde_json::Value;
use std::collections::BTreeSet;

/// 解析后的原生漏洞事实。来源字符串仅供内部归属，不应直接进入公开反馈。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CargoAuditFinding {
    /// RustSec 原生 advisory 标识。
    pub advisory_id: String,
    /// 锁文件中实际解析的包名。
    pub package_name: String,
    /// 锁文件中实际解析的版本。
    pub package_version: String,
    /// 锁文件中的依赖来源。
    pub package_source: String,
    /// 源码制品校验和；存在时保留用于后续归属。
    pub package_checksum: Option<String>,
    /// 原生 advisory 明确给出的 CVE 别名。
    pub cve_aliases: Vec<String>,
    /// 原生 CVSS 向量；缺失时不能推导严重度。
    pub cvss: Option<String>,
}

/// 一次 cargo-audit 报告的保守解析结果；数据库字段不构成可信身份。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CargoAuditObservation {
    /// 原生报告的锁文件依赖数量。
    pub dependency_count: u64,
    /// 原生报告的数据库 advisory 数量。
    pub database_advisory_count: u64,
    /// 原生报告声明的数据库提交，可能缺失且未核验。
    pub database_commit: Option<String>,
    /// 原生报告声明的数据库更新时刻，可能缺失且未核验。
    pub database_updated: Option<String>,
    /// 原生报告另有警告，公开消费者仍须保留未完成状态。
    pub warnings_present: bool,
    /// 逐个保留的漏洞发现。
    pub findings: Vec<CargoAuditFinding>,
}

/// 严格解析 cargo-audit 0.22 JSON，并核对无忽略规则及原生退出契约。
/// 参数为有界原始 JSON 和真实进程退出码；异常表示本次观察未完成。
pub fn parse_cargo_audit_json(
    bytes: &[u8],
    exit: Option<i32>,
) -> Result<CargoAuditObservation, &'static str> {
    let invalid = "cargo_audit_report_invalid";
    let report = parse_unique_json(bytes).map_err(|_| invalid)?;
    if report.as_object().is_none_or(|root| {
        root.len() != 5
            || [
                "database",
                "lockfile",
                "settings",
                "vulnerabilities",
                "warnings",
            ]
            .iter()
            .any(|field| !root.contains_key(*field))
    }) {
        return Err(invalid);
    }
    for field in [
        "database",
        "lockfile",
        "settings",
        "vulnerabilities",
        "warnings",
    ] {
        if !report[field].is_object() {
            return Err(invalid);
        }
    }
    let settings = &report["settings"];
    if !settings["ignore"].as_array().is_some_and(Vec::is_empty)
        || !settings["target_arch"]
            .as_array()
            .is_some_and(Vec::is_empty)
        || !settings["target_os"].as_array().is_some_and(Vec::is_empty)
        || !settings["severity"].is_null()
    {
        return Err("cargo_audit_suppressed_or_warning_unresolved");
    }
    let warnings_present = !report["warnings"].as_object().ok_or(invalid)?.is_empty();
    let dependency_count = report["lockfile"]["dependency-count"]
        .as_u64()
        .ok_or(invalid)?;
    let database_advisory_count = report["database"]["advisory-count"]
        .as_u64()
        .ok_or(invalid)?;
    let database_commit = optional_string(&report["database"]["last-commit"])?;
    let database_updated = optional_string(&report["database"]["last-updated"])?;
    let vulnerabilities = &report["vulnerabilities"];
    let found = vulnerabilities["found"].as_bool().ok_or(invalid)?;
    let count = vulnerabilities["count"].as_u64().ok_or(invalid)?;
    let list = vulnerabilities["list"].as_array().ok_or(invalid)?;
    if count != list.len() as u64
        || found != !list.is_empty()
        || exit != Some(if found { 1 } else { 0 })
    {
        return Err("cargo_audit_execution_incomplete");
    }
    let mut findings = Vec::with_capacity(list.len());
    let mut identities = BTreeSet::new();
    for item in list {
        let advisory = &item["advisory"];
        let package = &item["package"];
        let advisory_id = required_string(&advisory["id"])?;
        let advisory_package = required_string(&advisory["package"])?;
        let package_name = required_string(&package["name"])?;
        let package_version = required_string(&package["version"])?;
        let package_source = required_string(&package["source"])?;
        let package_checksum = optional_string(&package["checksum"])?;
        if !advisory_id.starts_with("RUSTSEC-")
            || advisory_package != package_name
            || semver::Version::parse(&package_version).is_err()
            || !package_source.starts_with("registry+") && !package_source.starts_with("git+")
            || !identities.insert((
                advisory_id.clone(),
                package_name.clone(),
                package_version.clone(),
                package_source.clone(),
            ))
        {
            return Err(invalid);
        }
        let aliases = advisory["aliases"].as_array().ok_or(invalid)?;
        let mut cve_aliases = BTreeSet::new();
        for alias in aliases {
            let alias = required_string(alias)?;
            if alias.starts_with("CVE-") {
                cve_aliases.insert(alias);
            }
        }
        findings.push(CargoAuditFinding {
            advisory_id,
            package_name,
            package_version,
            package_source,
            package_checksum,
            cve_aliases: cve_aliases.into_iter().collect(),
            cvss: optional_string(&advisory["cvss"])?,
        });
    }
    Ok(CargoAuditObservation {
        dependency_count,
        database_advisory_count,
        database_commit,
        database_updated,
        warnings_present,
        findings,
    })
}

/// 将原生 advisory 的组件、解析版本、来源和校验和绑定到本轮 Cargo.lock 字节。
/// 此比较不验证工具或数据库的可信来源。
pub fn bind_cargo_audit_lockfile(
    observation: &CargoAuditObservation,
    lock_bytes: &[u8],
) -> Result<(), &'static str> {
    if lock_bytes.is_empty() || lock_bytes.len() > 8 * 1024 * 1024 {
        return Err("cargo_audit_lock_invalid");
    }
    let lock_text = std::str::from_utf8(lock_bytes).map_err(|_| "cargo_audit_lock_invalid")?;
    let lock: toml::Value = toml::from_str(lock_text).map_err(|_| "cargo_audit_lock_invalid")?;
    let packages = lock
        .get("package")
        .and_then(toml::Value::as_array)
        .ok_or("cargo_audit_lock_invalid")?;
    if packages.len() as u64 != observation.dependency_count {
        return Err("cargo_audit_dependency_count_mismatch");
    }
    let mut resolved = BTreeSet::new();
    for package in packages {
        let item = package.as_table().ok_or("cargo_audit_lock_invalid")?;
        let name = item
            .get("name")
            .and_then(toml::Value::as_str)
            .ok_or("cargo_audit_lock_invalid")?;
        let version = item
            .get("version")
            .and_then(toml::Value::as_str)
            .ok_or("cargo_audit_lock_invalid")?;
        let source = item
            .get("source")
            .and_then(toml::Value::as_str)
            .unwrap_or("");
        let checksum = item
            .get("checksum")
            .and_then(toml::Value::as_str)
            .unwrap_or("");
        if !resolved.insert((name, version, source, checksum)) {
            return Err("cargo_audit_lock_ambiguous");
        }
    }
    for finding in &observation.findings {
        if !resolved.contains(&(
            finding.package_name.as_str(),
            finding.package_version.as_str(),
            finding.package_source.as_str(),
            finding.package_checksum.as_deref().unwrap_or(""),
        )) {
            return Err("cargo_audit_component_mismatch");
        }
    }
    Ok(())
}

fn required_string(value: &Value) -> Result<String, &'static str> {
    value
        .as_str()
        .filter(|text| {
            !text.is_empty() && text.len() <= 4096 && !text.chars().any(char::is_control)
        })
        .map(str::to_owned)
        .ok_or("cargo_audit_report_invalid")
}

fn optional_string(value: &Value) -> Result<Option<String>, &'static str> {
    if value.is_null() {
        Ok(None)
    } else {
        required_string(value).map(Some)
    }
}
