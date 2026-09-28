use codeguard_core::ObservedPathKind;
use serde_json::Value;

use crate::{EslintConfigState, EslintLocalCandidate, strict_json::parse_unique_json};

/// 只读核对项目声明、本地包身份、原生入口与配置状态；不搜索 PATH 或执行 JS。
/// 输入由调用方有界读取并分类；返回候选，不证明 Node 或 ESLint 本轮执行成功。
pub fn inspect_eslint_local_candidate(
    project_manifest: Option<&[u8]>,
    local_manifest: Option<&[u8]>,
    entry_kind: Option<ObservedPathKind>,
    config: EslintConfigState,
) -> EslintLocalCandidate {
    let declared_spec = match project_manifest.map(parse_project_declaration).transpose() {
        Ok(value) => value.flatten(),
        Err(()) => {
            return candidate(
                "project_manifest_invalid",
                None,
                None,
                "修正项目 package.json 的 JSON 或 ESLint 依赖声明后重查本地工具身份",
            );
        }
    };
    let observed_version = match local_manifest.map(parse_local_identity).transpose() {
        Ok(value) => value,
        Err(()) => {
            return candidate(
                "local_package_identity_invalid",
                declared_spec,
                None,
                "核对项目本地 ESLint package.json、版本和 bin 入口；不要把损坏制品说成源码违规",
            );
        }
    };
    if config == EslintConfigState::Invalid {
        return candidate(
            "configuration_invalid",
            declared_spec,
            observed_version,
            "修正项目原 ESLint 配置后用已发现的原生工具复检",
        );
    }
    if matches!(
        entry_kind,
        Some(ObservedPathKind::Symlink | ObservedPathKind::Directory | ObservedPathKind::Other)
    ) {
        return candidate(
            "local_entry_untrusted",
            declared_spec,
            observed_version,
            "核对项目本地 ESLint 入口的真实类型与来源，不跟随未知链接执行",
        );
    }
    if entry_kind == Some(ObservedPathKind::File) {
        return match observed_version {
            Some(version)
                if declared_spec
                    .as_deref()
                    .and_then(|spec| semver::Version::parse(spec).ok())
                    .is_some_and(|exact| exact.to_string() != version) =>
            {
                candidate(
                    "local_version_conflicts_with_declaration",
                    declared_spec,
                    Some(version),
                    "核对项目固定声明与本地 ESLint 包版本，按原项目依赖方案恢复一致后原生复检",
                )
            }
            Some(version) if supported_version(&version) => candidate(
                "local_candidate_requires_native_probe",
                declared_spec,
                Some(version),
                "用明确的 Node 路径、项目本地 ESLint 入口及原配置执行版本和规则核验",
            ),
            Some(version) => candidate(
                "local_version_not_supported_by_adapter",
                declared_spec,
                Some(version),
                "核对项目实际 ESLint 版本并补齐相应原生适配器，不按旧版结果放行",
            ),
            None => candidate(
                "local_entry_identity_unknown",
                declared_spec,
                None,
                "读取并核对项目本地 ESLint 包清单和入口版本后再执行，不根据 PATH 猜测",
            ),
        };
    }
    if declared_spec.is_some() || observed_version.is_some() {
        candidate(
            "declared_local_entry_unobserved",
            declared_spec,
            observed_version,
            "核对项目本地入口、包管理器布局或显式工具路径；不能仅凭 PATH 判定未安装",
        )
    } else {
        candidate(
            "unknown",
            None,
            None,
            "确认项目是否要求 ESLint，以及其本地或显式原生工具位置与配置",
        )
    }
}

fn parse_project_declaration(raw: &[u8]) -> Result<Option<String>, ()> {
    let object = parsed_object(raw)?;
    let mut declared: Option<&str> = None;
    for field in [
        "dependencies",
        "devDependencies",
        "optionalDependencies",
        "peerDependencies",
    ] {
        let Some(group) = object.get(field) else {
            continue;
        };
        let dependencies = group.as_object().ok_or(())?;
        if let Some(raw_spec) = dependencies.get("eslint") {
            let spec = raw_spec
                .as_str()
                .filter(|value| valid_spec(value))
                .ok_or(())?;
            if declared.is_some_and(|previous| previous != spec) {
                return Err(());
            }
            declared = Some(spec);
        }
    }
    Ok(declared.map(str::to_owned))
}

fn parse_local_identity(raw: &[u8]) -> Result<String, ()> {
    let object = parsed_object(raw)?;
    if object.get("name").and_then(Value::as_str) != Some("eslint") {
        return Err(());
    }
    let version = object.get("version").and_then(Value::as_str).ok_or(())?;
    let parsed = semver::Version::parse(version).map_err(|_| ())?;
    if !parsed.pre.is_empty() || !parsed.build.is_empty() {
        return Err(());
    }
    let entry = match object.get("bin") {
        Some(Value::String(path)) => Some(path.as_str()),
        Some(Value::Object(bins)) => bins.get("eslint").and_then(Value::as_str),
        _ => None,
    };
    if entry != Some("bin/eslint.js") {
        return Err(());
    }
    Ok(version.into())
}

fn parsed_object(raw: &[u8]) -> Result<serde_json::Map<String, Value>, ()> {
    if raw.is_empty() || raw.len() > 256 * 1024 {
        return Err(());
    }
    parse_unique_json(raw)
        .map_err(|_| ())?
        .as_object()
        .cloned()
        .ok_or(())
}

fn valid_spec(value: &str) -> bool {
    !value.trim().is_empty() && value.len() <= 256 && !value.chars().any(char::is_control)
}

fn supported_version(version: &str) -> bool {
    semver::Version::parse(version).is_ok_and(|parsed| parsed.major == 10)
}

fn candidate(
    state: &'static str,
    declared_spec: Option<String>,
    observed_version: Option<String>,
    next_action: &'static str,
) -> EslintLocalCandidate {
    EslintLocalCandidate {
        state,
        declared_spec,
        observed_version,
        next_action,
    }
}
