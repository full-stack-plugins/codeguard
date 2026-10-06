//! 工作区内JavaScript模块模式的有界静态观察；不执行项目配置或检查器。
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    io::ErrorKind,
    path::{Component, Path},
};

/// 精确源码及最近包的模式观察；只供候选规划与输入连续性核对，不授予质量权威。
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct JavascriptModeObservation {
    /// 观察协议版本。
    pub schema_version: String,
    /// 固定观察类型。
    pub report_type: String,
    /// module/commonjs/unknown；缺少明确依据必须unknown。
    pub mode: String,
    /// 文件后缀或最近包类型依据，不推断工具的生效配置。
    pub basis: String,
    /// 已校验的工作区相对源码路径；非法输入不输出原路径。
    pub source_path: Option<String>,
    /// 本轮普通UTF-8源码字节摘要。
    pub source_sha256: Option<String>,
    /// 最近package.json相对路径；不越过工作区。
    pub package_path: Option<String>,
    /// 最近普通清单字节摘要；坏JSON仍保留字节身份。
    pub package_sha256: Option<String>,
    /// 已搜索目录，包括不存在清单的负向依据，最多64个。
    pub searched_directories: Vec<String>,
    /// 具体观察或未解析原因，不作为源码违规。
    pub reason: String,
    /// 静态观察不执行原生工具。
    pub native_execution: String,
    /// 模式观察不决定质量交付。
    pub delivery_decision: String,
}

/// 在已选择的物理root中观察一个普通源码的声明模式。
/// 参数为绝对物理工作区和无跳级的相对源码路径；返回模式、摘要、搜索边界及unknown原因。
/// 两次独立观察可比较全部字段以发现源码/最近包/负向搜索依据变化；此比较不批准任务关闭。
pub fn observe_javascript_mode(root: &Path, relative: &Path) -> JavascriptModeObservation {
    let mut result = JavascriptModeObservation {
        schema_version: "0.1.0".into(),
        report_type: "javascript_mode_observation".into(),
        mode: "unknown".into(),
        basis: "undetermined".into(),
        source_path: None,
        source_sha256: None,
        package_path: None,
        package_sha256: None,
        searched_directories: Vec::new(),
        reason: "source_scope_invalid".into(),
        native_execution: "not_run".into(),
        delivery_decision: "not_evaluated".into(),
    };
    let Some(relative_text) = relative
        .to_str()
        .filter(|text| !text.is_empty() && !text.chars().any(char::is_control))
    else {
        return result;
    };
    if !root.is_absolute()
        || !root.is_dir()
        || root.canonicalize().ok().as_deref() != Some(root)
        || relative
            .components()
            .any(|part| !matches!(part, Component::Normal(_)))
    {
        return result;
    }
    result.source_path = Some(relative_text.to_owned());
    let source = root.join(relative);
    let bytes = match crate::plain_syntax_source::read_plain_source(&source) {
        Ok(bytes) => bytes,
        Err(reason) => {
            result.reason = reason.into();
            return result;
        }
    };
    if source.canonicalize().ok().as_deref() != Some(source.as_path()) {
        result.reason = "source_path_unbound".into();
        return result;
    }
    result.source_sha256 = Some(format!("{:x}", Sha256::digest(&bytes)));
    match relative
        .extension()
        .and_then(|extension| extension.to_str())
    {
        Some("mjs") => {
            result.mode = "module".into();
            result.basis = "extension_mjs".into();
            result.reason = "explicit_extension_mode_native_confirmation_required".into();
            return result;
        }
        Some("cjs") => {
            result.mode = "commonjs".into();
            result.basis = "extension_cjs".into();
            result.reason = "explicit_extension_mode_native_confirmation_required".into();
            return result;
        }
        Some("js") => {}
        _ => {
            result.reason = "source_extension_mode_unresolved".into();
            return result;
        }
    }
    let Some(parent) = source.parent() else {
        return result;
    };
    for (visited, directory) in parent
        .ancestors()
        .take_while(|path| path.starts_with(root))
        .enumerate()
    {
        if visited == 64 {
            result.reason = "package_search_budget_exhausted".into();
            return result;
        }
        let path = directory.strip_prefix(root).expect("bounded ancestor");
        let text = if path.as_os_str().is_empty() {
            "."
        } else {
            path.to_str().expect("validated UTF-8 relative source")
        };
        result.searched_directories.push(text.into());
        let package = directory.join("package.json");
        match std::fs::symlink_metadata(&package) {
            Err(error) if error.kind() == ErrorKind::NotFound => continue,
            Ok(metadata) if metadata.is_file() => {}
            _ => {
                result.reason = "package_path_untrusted".into();
                result.package_path = Some(if text == "." {
                    "package.json".into()
                } else {
                    format!("{text}/package.json")
                });
                return result;
            }
        }
        result.package_path = Some(if text == "." {
            "package.json".into()
        } else {
            format!("{text}/package.json")
        });
        result.basis = "nearest_package_type".into();
        let bytes = match codeguard_runtime::read_bounded_regular_file(&package, 256 * 1024) {
            Ok(bytes) => bytes,
            Err(_) => {
                result.reason = "package_bytes_unavailable_or_over_budget".into();
                return result;
            }
        };
        if package.canonicalize().ok().as_deref() != Some(package.as_path()) {
            result.reason = "package_path_untrusted".into();
            return result;
        }
        result.package_sha256 = Some(format!("{:x}", Sha256::digest(&bytes)));
        // 最近清单即形成包边界；坏清单或缺type不得继续继承外层声明。
        let manifest = match codeguard_adapters::parse_unique_json(&bytes) {
            Ok(value) if value.is_object() => value,
            _ => {
                result.reason = "package_json_invalid_or_ambiguous".into();
                return result;
            }
        };
        match manifest.get("type").and_then(serde_json::Value::as_str) {
            Some(mode @ ("module" | "commonjs")) => {
                result.mode = mode.into();
                result.reason = "explicit_package_type_native_confirmation_required".into();
            }
            _ => result.reason = "nearest_package_type_unresolved".into(),
        }
        return result;
    }
    result.reason = "package_type_not_observed".into();
    result
}
