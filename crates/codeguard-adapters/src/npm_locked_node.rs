use crate::{NpmAuditObservation, strict_json};
use std::collections::{BTreeMap, BTreeSet};

/// npm v3锁文件的普通安装节点；不声明依赖边或漏洞范围求值已经核验。
#[derive(Clone, Debug)]
pub struct NpmLockedNode {
    /// 锁文件内精确相对安装位置。
    pub location: String,
    /// 由安装位置取得并与显式名称核对的原生包名。
    pub native_name: String,
    /// 锁文件明确记载的完整语义版本，不使用审计范围猜测。
    pub resolved_version: String,
}

impl NpmLockedNode {
    /// 从有界v3锁文件字节读取节点；返回普通节点，链接及别名等待专用解析。
    pub fn parse(bytes: &[u8]) -> Result<Vec<Self>, &'static str> {
        if bytes.is_empty() || bytes.len() > 8 * 1024 * 1024 {
            return Err("npm_lock_size_invalid");
        }
        let value = strict_json::parse(bytes).map_err(|_| "npm_lock_json_invalid")?;
        if value.get("lockfileVersion").and_then(|v| v.as_u64()) != Some(3) {
            return Err("npm_lock_version_unsupported");
        }
        let packages = value
            .get("packages")
            .and_then(|v| v.as_object())
            .ok_or("npm_lock_packages_missing")?;
        if packages.len() > 10_001 || !packages.get("").is_some_and(|v| v.is_object()) {
            return Err("npm_lock_packages_invalid");
        }
        let mut nodes = Vec::new();
        for (location, raw) in packages {
            if location.is_empty() {
                continue;
            }
            let entry = raw.as_object().ok_or("npm_lock_node_invalid")?;
            if entry.contains_key("link") {
                return Err("npm_lock_link_requires_resolution");
            }
            let name = package_name(location).ok_or("npm_lock_location_invalid")?;
            if entry
                .get("name")
                .is_some_and(|v| v.as_str() != Some(name.as_str()))
            {
                return Err("npm_lock_alias_requires_resolution");
            }
            let version = entry
                .get("version")
                .and_then(|v| v.as_str())
                .ok_or("npm_lock_version_missing")?;
            if version.len() > 256 || semver::Version::parse(version).is_err() {
                return Err("npm_lock_resolved_version_invalid");
            }
            nodes.push(Self {
                location: location.clone(),
                native_name: name,
                resolved_version: version.into(),
            });
        }
        Ok(nodes)
    }

    /// 按本轮原生审计的全部节点位置关联明确版本；不批准漏洞或白名单。
    pub fn bind<'a>(
        nodes: &'a [Self],
        audit: &NpmAuditObservation,
    ) -> Result<Vec<&'a Self>, &'static str> {
        if nodes.iter().any(|node| {
            package_name(&node.location).as_deref() != Some(node.native_name.as_str())
                || node.resolved_version.len() > 256
                || semver::Version::parse(&node.resolved_version).is_err()
        }) {
            return Err("npm_audit_lock_identity_invalid");
        }
        let indexed: BTreeMap<&str, &Self> =
            nodes.iter().map(|n| (n.location.as_str(), n)).collect();
        if indexed.len() != nodes.len() || audit.native_dependency_total != nodes.len() as u64 {
            return Err("npm_audit_lock_inventory_mismatch");
        }
        let mut bound = Vec::new();
        let mut seen = BTreeSet::new();
        for component in &audit.components {
            for location in &component.node_locations {
                let node = indexed
                    .get(location.as_str())
                    .ok_or("npm_audit_lock_node_missing")?;
                if node.native_name != component.native_name || !seen.insert(location.as_str()) {
                    return Err("npm_audit_lock_identity_mismatch");
                }
                bound.push(*node);
            }
        }
        Ok(bound)
    }
}

fn package_name(location: &str) -> Option<String> {
    if location.len() > 4096 || location.contains('\\') {
        return None;
    }
    let parts: Vec<&str> = location.split('/').collect();
    let mut i = 0;
    let mut last = None;
    while i < parts.len() {
        if parts[i] != "node_modules" {
            return None;
        }
        i += 1;
        let first = *parts.get(i)?;
        if let Some(scope) = first.strip_prefix('@') {
            let second = *parts.get(i + 1)?;
            if !segment(scope) || !segment(second) {
                return None;
            }
            last = Some(format!("{first}/{second}"));
            i += 2;
        } else {
            if !segment(first) {
                return None;
            }
            last = Some(first.to_owned());
            i += 1;
        }
    }
    last
}

fn segment(value: &str) -> bool {
    !value.is_empty()
        && !matches!(value, "." | "..")
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"_.-".contains(&b))
}
