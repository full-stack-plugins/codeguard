//! Cargo 清单的静态成员和直接路径依赖观察；不执行 Cargo 有效模型解析。

use std::collections::{BTreeMap, BTreeSet};

/// 一份 Cargo.toml 的可定位声明；继承和条件缺口单独保留。
#[derive(Clone, Debug)]
pub struct CargoModuleModel {
    /// 直接声明的包名；虚拟工作区或非法声明没有此身份。
    pub package_name: Option<String>,
    /// 直接且符合 SemVer 的包版本；继承或缺失时不猜测默认版本。
    pub package_version: Option<String>,
    /// 无通配且无待解析排除条件的直接 workspace 成员。
    pub members: Vec<String>,
    /// 目标包名（已处理别名）、清单相对路径及 normal/build/dev 范围。
    pub path_dependencies: Vec<(String, String, String)>,
    /// 需要 Cargo 解析或原生模型才能确定的信息。
    pub unresolved: BTreeSet<String>,
    /// 仅直接声明的语言目标/方言；不是本机版本或有效构建模型。
    pub language_targets: BTreeMap<String, String>,
}

impl CargoModuleModel {
    /// 观察至多 256 KiB UTF-8 TOML；返回声明而非已解析依赖图。
    /// init 使用此方法不会运行构建脚本、生成锁文件或展开 Cargo workspace 继承。
    pub fn observe(bytes: &[u8]) -> Result<Self, &'static str> {
        if bytes.len() > 256 * 1024 {
            return Err("cargo_module_manifest_limit_exceeded");
        }
        let text = std::str::from_utf8(bytes).map_err(|_| "cargo_module_manifest_not_utf8")?;
        let value: toml::Value = text
            .parse()
            .map_err(|_| "cargo_module_manifest_invalid_toml")?;
        let root = value
            .as_table()
            .ok_or("cargo_module_manifest_invalid_root")?;
        let mut result = Self {
            package_name: None,
            package_version: None,
            members: Vec::new(),
            path_dependencies: Vec::new(),
            unresolved: BTreeSet::new(),
            language_targets: BTreeMap::new(),
        };
        if let Some(package) = root.get("package") {
            result.package_name = package
                .as_table()
                .and_then(|table| table.get("name"))
                .and_then(toml::Value::as_str)
                .filter(|name| valid_package_name(name))
                .map(str::to_owned);
            if let Some(package) = package.as_table() {
                if let Some(version) = package.get("version") {
                    result.package_version = version
                        .as_str()
                        .filter(|value| value.len() <= 256 && semver::Version::parse(value).is_ok())
                        .map(str::to_owned);
                    if result.package_version.is_none() {
                        result
                            .unresolved
                            .insert("package_version_unresolved".into());
                    }
                } else {
                    result
                        .unresolved
                        .insert("package_version_not_declared".into());
                }

                for key in ["rust-version", "edition"] {
                    let Some(raw) = package.get(key) else {
                        continue;
                    };
                    let value = raw.as_str().filter(|value| valid_rust_target(value, key));
                    if let Some(value) = value {
                        result.language_targets.insert(key.into(), value.into());
                    } else {
                        result
                            .unresolved
                            .insert(format!("language_target_unresolved:{key}"));
                    }
                }
            }
            if result.package_name.is_none() {
                result.unresolved.insert("package_name_unresolved".into());
            }
        }
        if let Some(workspace) = root.get("workspace") {
            if let Some(workspace) = workspace.as_table() {
                if workspace.contains_key("dependencies") {
                    result
                        .unresolved
                        .insert("workspace_dependencies_not_resolved".into());
                }
                let excludes = workspace.get("exclude");
                let exclusions_unresolved = excludes
                    .is_some_and(|value| value.as_array().is_none_or(|array| !array.is_empty()));
                if exclusions_unresolved {
                    result
                        .unresolved
                        .insert("workspace_exclusions_not_resolved".into());
                }
                if let Some(members) = workspace.get("members") {
                    if let Some(members) = members.as_array() {
                        for member in members {
                            if let Some(member) = member.as_str().filter(|member| {
                                valid_path_text(member) && !member.contains(['*', '?', '[', ']'])
                            }) {
                                if !exclusions_unresolved {
                                    result.members.push(member.into());
                                }
                            } else {
                                result
                                    .unresolved
                                    .insert("workspace_member_unresolved".into());
                            }
                        }
                    } else {
                        result
                            .unresolved
                            .insert("workspace_members_unresolved".into());
                    }
                }
            } else {
                result
                    .unresolved
                    .insert("workspace_declaration_invalid".into());
            }
        }
        for key in ["target", "features", "patch", "replace"] {
            if root.contains_key(key) {
                result.unresolved.insert(format!("{key}_not_resolved"));
            }
        }
        for (key, scope) in [
            ("dependencies", "normal"),
            ("build-dependencies", "build"),
            ("dev-dependencies", "dev"),
        ] {
            let Some(dependencies) = root.get(key) else {
                continue;
            };
            let Some(dependencies) = dependencies.as_table() else {
                result.unresolved.insert("dependency_table_invalid".into());
                continue;
            };
            for (alias, value) in dependencies {
                let Some(table) = value.as_table() else {
                    result
                        .unresolved
                        .insert("external_dependency_not_resolved".into());
                    continue;
                };
                if table.contains_key("workspace") {
                    result
                        .unresolved
                        .insert("dependency_workspace_not_resolved".into());
                    continue;
                }
                if table
                    .get("optional")
                    .is_some_and(|value| value.as_bool() != Some(false))
                {
                    result
                        .unresolved
                        .insert("optional_dependency_not_resolved".into());
                    continue;
                }
                if table.contains_key("git")
                    || table.contains_key("registry")
                    || table.contains_key("registry-index")
                {
                    result
                        .unresolved
                        .insert("dependency_source_not_resolved".into());
                    continue;
                }
                let Some(path) = table
                    .get("path")
                    .and_then(toml::Value::as_str)
                    .filter(|path| valid_path_text(path))
                else {
                    result
                        .unresolved
                        .insert("dependency_path_not_resolved".into());
                    continue;
                };
                let package = match table.get("package") {
                    Some(value) => value.as_str(),
                    None => Some(alias.as_str()),
                };
                let Some(package) = package.filter(|name| valid_package_name(name)) else {
                    result
                        .unresolved
                        .insert("dependency_package_unresolved".into());
                    continue;
                };
                if table.contains_key("version") {
                    result
                        .unresolved
                        .insert("dependency_version_not_resolved".into());
                }
                if table.contains_key("features") || table.contains_key("default-features") {
                    result
                        .unresolved
                        .insert("dependency_features_not_resolved".into());
                }
                result
                    .path_dependencies
                    .push((package.into(), path.into(), scope.into()));
            }
        }
        Ok(result)
    }
}
fn valid_package_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 128
        && name
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
}
fn valid_path_text(path: &str) -> bool {
    !path.is_empty() && path.len() <= 4096 && !path.chars().any(char::is_control)
}

fn valid_rust_target(value: &str, key: &str) -> bool {
    if key == "edition" {
        return matches!(value, "2015" | "2018" | "2021" | "2024");
    }
    let parts: Vec<_> = value.split('.').collect();
    value.len() <= 16
        && matches!(parts.len(), 2 | 3)
        && parts
            .iter()
            .all(|part| !part.is_empty() && part.bytes().all(|byte| byte.is_ascii_digit()))
}
