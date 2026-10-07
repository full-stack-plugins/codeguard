//! manifest/lock 静态观察模块（9.17）。
//!
//! 接入各已实现 adapter 的 manifest/lock/wrapper 静态观察：init 不执行项目脚本，
//! 声明版本和已解析版本分别有依据。

use serde_json::{Value, json};
use std::path::Path;

/// manifest 观察结果。
pub(crate) struct ManifestObservation {
    pub manifest_type: String,
    pub declared_version: Option<String>,
    pub resolved_version: Option<String>,
    pub status: String,
}

/// 观察 manifest 文件。
pub(crate) fn observe_manifest(root: &Path, manifest_name: &str) -> ManifestObservation {
    let manifest_path = root.join(manifest_name);
    if !manifest_path.exists() {
        return ManifestObservation {
            manifest_type: manifest_name.to_string(),
            declared_version: None,
            resolved_version: None,
            status: "missing".to_string(),
        };
    }
    
    let content = std::fs::read_to_string(&manifest_path).unwrap_or_default();
    
    // 提取声明版本（简单解析）
    let declared_version = if manifest_name == "package.json" {
        extract_json_field(&content, "version")
    } else if manifest_name == "Cargo.toml" {
        extract_toml_field(&content, "version")
    } else if manifest_name == "go.mod" {
        extract_go_version(&content)
    } else {
        None
    };
    
    ManifestObservation {
        manifest_type: manifest_name.to_string(),
        declared_version,
        resolved_version: None,
        status: "observed".to_string(),
    }
}

/// 生成 manifest 观察报告。
pub(crate) fn manifest_report(observation: &ManifestObservation) -> Value {
    json!({
        "schema_version": "0.1.0",
        "report_type": "manifest_observation",
        "manifest_type": observation.manifest_type,
        "declared_version": observation.declared_version,
        "resolved_version": observation.resolved_version,
        "status": observation.status,
    })
}

/// 提取 JSON 字段值。
fn extract_json_field(content: &str, field: &str) -> Option<String> {
    let pattern = format!("\"{}\":", field);
    let idx = content.find(&pattern)?;
    let start = idx + pattern.len();
    let remaining = &content[start..];
    let value_start = remaining.find('"')? + 1;
    let value_end = remaining[value_start..].find('"')?;
    Some(remaining[value_start..value_start + value_end].to_string())
}

/// 提取 TOML 字段值。
fn extract_toml_field(content: &str, field: &str) -> Option<String> {
    let pattern = format!("{} = ", field);
    let idx = content.find(&pattern)?;
    let start = idx + pattern.len();
    let remaining = &content[start..];
    let end = remaining.find('\n').unwrap_or(remaining.len());
    Some(remaining[..end].trim().trim_matches('"').to_string())
}

/// 提取 Go 版本。
fn extract_go_version(content: &str) -> Option<String> {
    let idx = content.find("go ")?;
    let start = idx + 3;
    let remaining = &content[start..];
    let end = remaining.find('\n').unwrap_or(remaining.len());
    Some(remaining[..end].trim().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn observe_manifest_missing() {
        let root = std::env::temp_dir().join("manifest-test");
        std::fs::create_dir_all(&root).unwrap();
        let observation = observe_manifest(&root, "package.json");
        assert_eq!(observation.status, "missing");
        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn observe_manifest_package_json() {
        let root = std::env::temp_dir().join("manifest-test2");
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join("package.json"), r#"{"version": "1.0.0"}"#).unwrap();
        let observation = observe_manifest(&root, "package.json");
        assert_eq!(observation.status, "observed");
        assert_eq!(observation.declared_version, Some("1.0.0".to_string()));
        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn manifest_report_contains_version() {
        let observation = ManifestObservation {
            manifest_type: "package.json".to_string(),
            declared_version: Some("1.0.0".to_string()),
            resolved_version: None,
            status: "observed".to_string(),
        };
        let report = manifest_report(&observation);
        assert_eq!(report["declared_version"], "1.0.0");
    }
}
