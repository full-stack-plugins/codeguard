//! PEP 751 标准锁的保守组件身份提取；未解析的环境、extras 和依赖组拒绝归属。

use crate::PipAuditObservation;
use std::collections::BTreeSet;

/// 从标准 pylock 提取每个显式版本组件的规范化身份。
/// 无版本、重复身份或必要字段缺失时拒绝本轮归属候选，不声称完整环境选择。
pub fn parse_pylock_package_identities(
    bytes: &[u8],
) -> Result<BTreeSet<(String, String)>, &'static str> {
    if bytes.is_empty() || bytes.len() > 8 * 1024 * 1024 {
        return Err("python_standard_lock_invalid");
    }
    let text = std::str::from_utf8(bytes).map_err(|_| "python_standard_lock_invalid")?;
    let lock: toml::Value = text.parse().map_err(|_| "python_standard_lock_invalid")?;
    let root = lock.as_table().ok_or("python_standard_lock_invalid")?;
    if root.get("lock-version").and_then(toml::Value::as_str) != Some("1.0")
        || root
            .get("created-by")
            .and_then(toml::Value::as_str)
            .is_none_or(str::is_empty)
    {
        return Err("python_standard_lock_invalid");
    }
    // 锁允许按环境、extra 和组选择不同包；无选择上下文时不能把所有包合并为项目结果。
    if root.contains_key("environments") || root.contains_key("requires-python") {
        return Err("python_lock_selection_unresolved");
    }
    for key in ["extras", "dependency-groups", "default-groups"] {
        if let Some(value) = root.get(key) {
            let choices = value.as_array().ok_or("python_standard_lock_invalid")?;
            if !choices.iter().all(|choice| choice.as_str().is_some()) {
                return Err("python_standard_lock_invalid");
            }
            if !choices.is_empty() {
                return Err("python_lock_selection_unresolved");
            }
        }
    }
    let packages = root
        .get("packages")
        .and_then(toml::Value::as_array)
        .filter(|packages| packages.len() <= 10_000)
        .ok_or("python_standard_lock_invalid")?;
    let mut identities = BTreeSet::new();
    for package in packages {
        let package = package.as_table().ok_or("python_standard_lock_invalid")?;
        if package.contains_key("marker") || package.contains_key("requires-python") {
            return Err("python_lock_selection_unresolved");
        }
        let name = package
            .get("name")
            .and_then(toml::Value::as_str)
            .filter(|name| valid_name(name))
            .ok_or("python_standard_lock_invalid")?;
        let version = package
            .get("version")
            .and_then(toml::Value::as_str)
            .filter(|version| valid_version(version))
            .ok_or("pylock_package_version_unresolved")?;
        if !identities.insert((canonical_name(name), version.to_owned())) {
            return Err("python_standard_lock_ambiguous");
        }
    }
    Ok(identities)
}

/// 要求原生报告的整组组件名和版本与本轮标准锁完全一致；环境选择仍未证明。
pub fn bind_pip_audit_lockfile(
    observation: &PipAuditObservation,
    locked: &BTreeSet<(String, String)>,
) -> Result<(), &'static str> {
    let observed: BTreeSet<_> = observation
        .dependencies
        .iter()
        .map(|dependency| (canonical_name(&dependency.name), dependency.version.clone()))
        .collect();
    if observed == *locked {
        Ok(())
    } else {
        Err("pip_audit_lock_attribution_unverified")
    }
}

fn valid_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 214
        && name
            .bytes()
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

fn valid_version(version: &str) -> bool {
    !version.is_empty()
        && version.len() <= 128
        && version.bytes().all(|byte| {
            byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'-' | b'_' | b'+' | b'!')
        })
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
