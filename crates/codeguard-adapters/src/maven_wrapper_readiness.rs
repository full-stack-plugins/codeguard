use codeguard_core::ObservedPathKind;

use crate::MavenWrapperCandidate;

/// 只读核对 Maven Wrapper 脚本和配置；不查 PATH、不执行脚本，也不下载发行包。
/// 参数是调用方分类后的脚本类型及有界读取的 properties；返回候选而非检查通过。
pub fn inspect_maven_wrapper_candidate(
    script_kind: Option<ObservedPathKind>,
    properties: Option<&[u8]>,
) -> MavenWrapperCandidate {
    let version = match properties.map(parse_distribution_version).transpose() {
        Ok(value) => value,
        Err(()) => {
            return candidate(
                "wrapper_configuration_invalid",
                None,
                "修正 Maven Wrapper 的 distributionUrl 配置，再核对原生 Maven 版本",
            );
        }
    };
    if matches!(
        script_kind,
        Some(ObservedPathKind::Symlink | ObservedPathKind::Directory | ObservedPathKind::Other)
    ) {
        return candidate(
            "wrapper_script_untrusted",
            version,
            "核对 Maven Wrapper 脚本的真实类型与来源，不跟随未知链接执行",
        );
    }
    match (script_kind, version) {
        (Some(ObservedPathKind::File), Some(version)) => {
            let state = if semver::Version::parse(&version).is_ok_and(|parsed| parsed.major == 3) {
                "wrapper_candidate_requires_native_probe"
            } else {
                "wrapper_version_not_supported_by_adapter"
            };
            let action = if state == "wrapper_candidate_requires_native_probe" {
                "以受控方式执行项目 Maven Wrapper 的版本探针，再运行项目原生检查"
            } else {
                "核对项目 Maven 版本并补齐相应适配器，不按旧版结果放行"
            };
            candidate(state, Some(version), action)
        }
        (None, Some(version)) => candidate(
            "wrapper_script_unobserved",
            Some(version),
            "核对项目 Maven Wrapper 脚本或显式 Maven 路径，不能仅凭 PATH 判定未安装",
        ),
        (Some(ObservedPathKind::File), None) => candidate(
            "wrapper_configuration_unobserved",
            None,
            "核对 Maven Wrapper 配置或显式 Maven 路径，不能仅凭 PATH 判定未安装",
        ),
        (None, None) => candidate(
            "unknown",
            None,
            "确认项目构建器、Wrapper 和显式 Maven 路径，再判断原生检查准备状态",
        ),
        _ => unreachable!("已处理全部非文件脚本类型"),
    }
}

fn parse_distribution_version(raw: &[u8]) -> Result<String, ()> {
    if raw.is_empty() || raw.len() > 64 * 1024 {
        return Err(());
    }
    let text = std::str::from_utf8(raw).map_err(|_| ())?;
    let mut distribution_url: Option<&str> = None;
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with('!') {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        if key.trim() == "distributionUrl" {
            if distribution_url.is_some() {
                return Err(());
            }
            distribution_url = Some(value.trim());
        }
    }
    let url = distribution_url.ok_or(())?.replace("\\:", ":");
    let filename = url.rsplit('/').next().ok_or(())?;
    let without_prefix = filename.strip_prefix("apache-maven-").ok_or(())?;
    let version = without_prefix
        .strip_suffix("-bin.zip")
        .or_else(|| without_prefix.strip_suffix("-bin.tar.gz"))
        .ok_or(())?;
    let parsed = semver::Version::parse(version).map_err(|_| ())?;
    if !parsed.pre.is_empty() || !parsed.build.is_empty() {
        return Err(());
    }
    Ok(version.to_owned())
}

fn candidate(
    state: &'static str,
    observed_version: Option<String>,
    next_action: &'static str,
) -> MavenWrapperCandidate {
    MavenWrapperCandidate {
        state,
        observed_version,
        next_action,
    }
}
