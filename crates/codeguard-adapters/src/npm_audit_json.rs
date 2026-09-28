//! npm 11 auditReportVersion2观察；不自行扫描CVE或推断已解析版本。
use crate::{NpmAuditComponent, NpmAuditObservation, strict_json};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
/// 核对有界原生JSON、同一具体npm11版本及冻结audit-level=info退出契约。
/// 参数为stdout字节、预期/实测版本和真实退出码；异常返回未完成原因。
pub fn parse_npm_audit_json(
    bytes: &[u8],
    expected: &str,
    observed: &str,
    exit: Option<i32>,
) -> Result<NpmAuditObservation, &'static str> {
    let invalid = "npm_audit_report_invalid";
    if expected != observed
        || semver::Version::parse(expected)
            .ok()
            .is_none_or(|v| v.major != 11 || !v.pre.is_empty() || !v.build.is_empty())
    {
        return Err("npm_version_unverified");
    }
    if bytes.is_empty() || bytes.len() > 8 * 1024 * 1024 {
        return Err("npm_audit_report_size_invalid");
    }
    let r = strict_json::parse(bytes).map_err(|_| invalid)?;
    if r.get("error").is_some() {
        return Err("npm_native_error");
    }
    if !exact(&r, &["auditReportVersion", "vulnerabilities", "metadata"])
        || r["auditReportVersion"] != 2
    {
        return Err(invalid);
    }
    let vulns = r["vulnerabilities"]
        .as_object()
        .filter(|v| v.len() <= 10_000)
        .ok_or(invalid)?;
    let meta = &r["metadata"];
    if !exact(meta, &["vulnerabilities", "dependencies"])
        || !exact(
            &meta["vulnerabilities"],
            &["info", "low", "moderate", "high", "critical", "total"],
        )
        || !exact(
            &meta["dependencies"],
            &["prod", "dev", "optional", "peer", "peerOptional", "total"],
        )
    {
        return Err(invalid);
    }
    if meta["dependencies"]
        .as_object()
        .unwrap()
        .values()
        .any(|v| v.as_u64().is_none_or(|n| n > 1_000_000))
    {
        return Err(invalid);
    }
    let mut counts = BTreeMap::from([
        ("info", 0u64),
        ("low", 0),
        ("moderate", 0),
        ("high", 0),
        ("critical", 0),
    ]);
    let mut components = Vec::new();
    for (name, v) in vulns {
        if !package(name)
            || !exact(
                v,
                &[
                    "name",
                    "severity",
                    "isDirect",
                    "via",
                    "effects",
                    "range",
                    "nodes",
                    "fixAvailable",
                ],
            )
            || v["name"] != *name
        {
            return Err(invalid);
        }
        let sev = text(&v["severity"], 32).ok_or(invalid)?;
        let count = counts.get_mut(sev).ok_or(invalid)?;
        *count += 1;
        let range = text(&v["range"], 4096).ok_or(invalid)?;
        let direct = v["isDirect"].as_bool().ok_or(invalid)?;
        let locations = strings(&v["nodes"])?;
        if locations.is_empty() || locations.iter().any(|p| !location(p)) {
            return Err(invalid);
        }
        let fix = &v["fixAvailable"];
        if !fix.is_boolean()
            && !(exact(fix, &["name", "version", "isSemVerMajor"])
                && fix["name"].as_str().is_some_and(package)
                && fix["version"]
                    .as_str()
                    .is_some_and(|s| semver::Version::parse(s).is_ok())
                && fix["isSemVerMajor"].is_boolean())
        {
            return Err(invalid);
        }
        for effect in strings(&v["effects"])? {
            if !vulns.contains_key(&effect) {
                return Err(invalid);
            }
        }
        let via = v["via"]
            .as_array()
            .filter(|v| !v.is_empty() && v.len() <= 10_000)
            .ok_or(invalid)?;
        let mut sources = BTreeSet::new();
        let mut related = BTreeSet::new();
        for advisory in via {
            if let Some(n) = advisory.as_str() {
                if !vulns.contains_key(n) || !related.insert(n.to_owned()) {
                    return Err(invalid);
                }
            } else {
                let source = advisory["source"]
                    .as_u64()
                    .filter(|n| *n > 0)
                    .ok_or(invalid)?;
                if !advisory.is_object()
                    || advisory["name"] != *name
                    || advisory["dependency"] != *name
                    || !sources.insert(source)
                    || !counts.contains_key(advisory["severity"].as_str().unwrap_or(""))
                    || severity_rank(sev)
                        < severity_rank(advisory["severity"].as_str().unwrap_or(""))
                    || text(&advisory["range"], 4096).is_none()
                    || text(&advisory["title"], 8192).is_none()
                    || text(&advisory["url"], 4096).is_none()
                {
                    return Err(invalid);
                }
            }
        }
        components.push(NpmAuditComponent {
            native_name: name.clone(),
            severity: sev.into(),
            is_direct: direct,
            affected_range: range.into(),
            node_locations: locations,
            advisory_sources: sources.into_iter().collect(),
            via_components: related.into_iter().collect(),
        });
    }
    if counts
        .iter()
        .any(|(k, n)| meta["vulnerabilities"][*k].as_u64() != Some(*n))
        || meta["vulnerabilities"]["total"].as_u64() != Some(vulns.len() as u64)
    {
        return Err(invalid);
    }
    if exit != Some(if vulns.is_empty() { 0 } else { 1 }) {
        return Err("npm_audit_execution_incomplete");
    }
    Ok(NpmAuditObservation {
        advisory_coverage: "not_evaluated",
        native_dependency_total: meta["dependencies"]["total"].as_u64().unwrap(),
        components,
    })
}
fn severity_rank(value: &str) -> usize {
    ["info", "low", "moderate", "high", "critical"]
        .iter()
        .position(|s| *s == value)
        .unwrap_or(usize::MAX)
}
fn exact(v: &Value, keys: &[&str]) -> bool {
    v.as_object()
        .is_some_and(|m| m.len() == keys.len() && keys.iter().all(|k| m.contains_key(*k)))
}
fn text(v: &Value, max: usize) -> Option<&str> {
    v.as_str()
        .filter(|s| !s.is_empty() && s.len() <= max && !s.chars().any(char::is_control))
}
fn package(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 214
        && s.bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"@/_.-".contains(&b))
        && !s.contains("..")
}
fn strings(v: &Value) -> Result<Vec<String>, &'static str> {
    let values = v
        .as_array()
        .filter(|v| v.len() <= 10_000)
        .ok_or("npm_audit_report_invalid")?;
    let mut seen = BTreeSet::new();
    values
        .iter()
        .map(|v| {
            let s = text(v, 4096).ok_or("npm_audit_report_invalid")?;
            if !seen.insert(s) {
                return Err("npm_audit_report_invalid");
            }
            Ok(s.to_owned())
        })
        .collect()
}
fn location(p: &str) -> bool {
    !p.is_empty()
        && !p.starts_with('/')
        && !p.contains('\\')
        && !p.split('/').any(|p| p.is_empty() || p == "." || p == "..")
}
