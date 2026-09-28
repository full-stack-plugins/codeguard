//! 将 Java 源码归属到最近构建根，汇总未接入原生检查器的配置状态。

use std::collections::BTreeSet;

use codeguard_adapters::CheckerConfiguration;

/// 单个已知 Maven 检查器在所有目标 Java 源码上的保守状态。
pub(crate) struct JavaCheckerCandidate {
    pub checker_id: Option<&'static str>,
    pub status: &'static str,
    pub reason: &'static str,
    pub next_action: &'static str,
}

/// 返回依赖、CVE 或安全类别对应的已识别 Maven 检查器；不证明其它检查器不存在。
pub(crate) fn checker_for_category(category: &str) -> Option<&'static str> {
    match category {
        "dependencies" => Some("java.maven.dependency"),
        "cve" => Some("java.maven.dependency_check"),
        "security" => Some("java.maven.findsecbugs"),
        _ => None,
    }
}

/// 按每份源码最近的构建根汇总，防止一处配置替整仓所有模块宣称已配置。
pub(crate) fn summarize(
    checker_id: &'static str,
    sources: &BTreeSet<String>,
    configurations: &[CheckerConfiguration],
) -> JavaCheckerCandidate {
    let mut statuses = BTreeSet::new();
    let gradle_id = checker_id.replace("java.maven.", "java.gradle.");
    let mut maven_seen = false;
    let mut gradle_seen = false;
    for source in sources {
        let candidates: Vec<_> = configurations
            .iter()
            .filter(|entry| {
                (entry.checker_id == checker_id || entry.checker_id == gradle_id)
                    && (entry.build_root == "."
                        || source.starts_with(&format!("{}/", entry.build_root)))
            })
            .collect();
        let longest = candidates.iter().map(|entry| entry.build_root.len()).max();
        let closest: Vec<_> = candidates
            .into_iter()
            .filter(|entry| Some(entry.build_root.len()) == longest)
            .collect();
        let status = match closest.as_slice() {
            [entry] if entry.checker_id == checker_id => {
                maven_seen = true;
                entry.configuration.as_str()
            }
            [entry] if entry.checker_id == gradle_id => {
                gradle_seen = true;
                "gradle_unknown"
            }
            entries => {
                maven_seen |= entries.iter().any(|entry| entry.checker_id == checker_id);
                gradle_seen |= entries.iter().any(|entry| entry.checker_id == gradle_id);
                "unobserved"
            }
        };
        statuses.insert(status);
    }
    let (status, reason, next_action) = if maven_seen && gradle_seen {
        (
            "configuration_unresolved",
            "checker_build_systems_mixed",
            "逐构建根核对 Maven 与 Gradle 的检查器配置",
        )
    } else if gradle_seen {
        (
            "configuration_unresolved",
            "gradle_model_not_resolved",
            "解析目标构建根的 Gradle 生效模型后重新检查",
        )
    } else if statuses.len() == 1 && statuses.contains("configured") {
        (
            "native_incomplete",
            "configured_checker_not_integrated",
            "接入并运行已声明的原生检查器，解析本次报告",
        )
    } else if statuses.len() == 1 && statuses.contains("missing") {
        (
            "not_configured",
            "recognized_checker_not_configured",
            "确认批准策略是否要求该检查器；需要时配置后重新探测",
        )
    } else if statuses.is_empty() || statuses.contains("unobserved") {
        (
            "configuration_unresolved",
            "checker_configuration_not_observed",
            "补齐目标构建根的配置观察后重新检查",
        )
    } else if statuses.len() > 1 {
        (
            "configuration_unresolved",
            "checker_configuration_mixed_across_sources",
            "逐构建根核对缺失或未知配置，再运行原生检查",
        )
    } else {
        (
            "configuration_unresolved",
            "checker_configuration_unresolved",
            "解析当前构建根的生效检查器配置",
        )
    };
    JavaCheckerCandidate {
        checker_id: (maven_seen && !gradle_seen && !statuses.contains("unobserved"))
            .then_some(checker_id),
        status,
        reason,
        next_action,
    }
}
