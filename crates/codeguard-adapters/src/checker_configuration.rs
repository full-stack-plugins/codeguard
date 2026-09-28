//! 从原生配置静态识别检查器声明；不执行项目命令或推断动态生效模型。

use roxmltree::{Document, Node, ParsingOptions};
use serde::Serialize;

const CHECKERS: [(&str, &str, &str, &str); 7] = [
    (
        "java.maven.pmd",
        "lint",
        "org.apache.maven.plugins",
        "maven-pmd-plugin",
    ),
    (
        "java.maven.p3c",
        "lint",
        "org.apache.maven.plugins",
        "maven-pmd-plugin",
    ),
    (
        "java.maven.checkstyle",
        "lint",
        "org.apache.maven.plugins",
        "maven-checkstyle-plugin",
    ),
    (
        "java.maven.javadoc",
        "comments",
        "org.apache.maven.plugins",
        "maven-javadoc-plugin",
    ),
    (
        "java.maven.dependency",
        "dependencies",
        "org.apache.maven.plugins",
        "maven-dependency-plugin",
    ),
    (
        "java.maven.dependency_check",
        "cve",
        "org.owasp",
        "dependency-check-maven",
    ),
    (
        "java.maven.findsecbugs",
        "security",
        "com.github.spotbugs",
        "spotbugs-maven-plugin",
    ),
];

// 仅识别已由本地原生哨兵核对过的 P3C 2.1.1 制品内规则集名称；不推断执行覆盖。
const P3C_2_1_1_RULESETS: [&str; 10] = [
    "rulesets/java/ali-comment.xml",
    "rulesets/java/ali-concurrent.xml",
    "rulesets/java/ali-constant.xml",
    "rulesets/java/ali-exception.xml",
    "rulesets/java/ali-flowcontrol.xml",
    "rulesets/java/ali-naming.xml",
    "rulesets/java/ali-oop.xml",
    "rulesets/java/ali-orm.xml",
    "rulesets/java/ali-other.xml",
    "rulesets/java/ali-set.xml",
];

/// 项目构建根中某检查器的静态配置状态；不表示本次已经执行。
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct CheckerConfiguration {
    pub build_root: String,
    pub checker_id: String,
    pub category: String,
    pub configuration: String,
    pub configuration_ref: String,
    pub reason: String,
    pub next_action: String,
}

/// POM 无法读取时保留检查器候选，但不将 I/O 故障说成配置错误。
pub fn inspect_maven_unreadable(build_root: &str, source: &str) -> Vec<CheckerConfiguration> {
    CHECKERS
        .iter()
        .map(|(id, category, _, _)| {
            entry(
                build_root,
                source,
                id,
                category,
                "unknown",
                "pom_unreadable",
                "恢复 POM 读取权限或缩小文件后重新探测",
            )
        })
        .collect()
}

/// 读取当前 POM 的直接插件声明；父 POM、profile 与 pluginManagement 仅记未知。
pub fn inspect_maven_pom(
    bytes: &[u8],
    build_root: &str,
    source: &str,
) -> Vec<CheckerConfiguration> {
    let parsed = std::str::from_utf8(bytes).ok().and_then(|text| {
        Document::parse_with_options(
            text,
            ParsingOptions {
                allow_dtd: false,
                nodes_limit: 100_000,
                entity_resolver: None,
            },
        )
        .ok()
    });
    let Some(document) = parsed.as_ref() else {
        return CHECKERS
            .iter()
            .map(|(id, category, _, _)| {
                entry(
                    build_root,
                    source,
                    id,
                    category,
                    "invalid",
                    "pom_invalid_xml",
                    "修正 POM XML 后重新探测",
                )
            })
            .collect();
    };
    let root = document.root_element();
    if !named(root, "project") {
        return CHECKERS
            .iter()
            .map(|(id, category, _, _)| {
                entry(
                    build_root,
                    source,
                    id,
                    category,
                    "invalid",
                    "pom_root_not_project",
                    "修正 Maven POM 根元素后重新探测",
                )
            })
            .collect();
    }
    let inherited = child(root, "parent").is_some();
    let managed = child(root, "build")
        .and_then(|build| child(build, "pluginManagement"))
        .and_then(|management| child(management, "plugins"));
    let direct = child(root, "build").and_then(|build| child(build, "plugins"));
    let unresolved_direct_identity = direct.is_some_and(has_unresolved_plugin_identity);
    let profiles = child(root, "profiles");
    CHECKERS
        .iter()
        .map(|(id, category, group, artifact)| {
            let direct_plugin = direct.and_then(|plugins| find_plugin(plugins, group, artifact));
            let declared = direct_plugin.is_some_and(|plugin| {
                *id != "java.maven.findsecbugs" || has_findsecbugs_dependency(plugin)
            });
            let possible_security = *id == "java.maven.findsecbugs" && direct_plugin.is_some();
            let managed_only = managed
                .and_then(|plugins| find_plugin(plugins, group, artifact))
                .is_some();
            let profile_only = profiles.is_some_and(|profiles| {
                profiles
                    .descendants()
                    .any(|node| named(node, "plugin") && plugin_identity(node, group, artifact))
            });
            let skip = direct_plugin.and_then(plugin_skip_value).or_else(|| {
                (*id == "java.maven.javadoc")
                    .then(|| child(root, "properties"))
                    .flatten()
                    .and_then(|properties| child(properties, "maven.javadoc.skip"))
                    .and_then(|value| value.text())
                    .map(str::trim)
            });
            let (status, reason, action) = if *id == "java.maven.p3c" {
                if let Some(plugin) = direct_plugin {
                    let p3c = inspect_p3c_plugin(plugin);
                    if p3c.0 == "missing" && (inherited || managed_only || profile_only) {
                        (
                            "unknown",
                            "p3c_effective_model_not_resolved",
                            "解析父 POM、pluginManagement 与 profile 后确认 P3C 配置",
                        )
                    } else {
                        p3c
                    }
                } else if managed_only || profile_only || inherited || unresolved_direct_identity {
                    (
                        "unknown",
                        "effective_model_or_profile_not_resolved",
                        "解析生效 Maven 模型以确定 P3C 规则集和插件依赖",
                    )
                } else {
                    (
                        "missing",
                        "p3c_plugin_not_declared",
                        "检查是否需要在 Maven PMD 中声明 P3C 制品和规则集",
                    )
                }
            } else if declared && skip == Some("true") {
                (
                    "invalid",
                    "plugin_explicitly_skipped",
                    "检查并修正插件 skip 配置后重新探测",
                )
            } else if declared && skip.is_some_and(|value| value.contains("${")) {
                (
                    "unknown",
                    "plugin_skip_value_dynamic",
                    "解析生效 Maven 属性以确定插件是否跳过",
                )
            } else if *id == "java.maven.javadoc" && declared {
                inspect_javadoc_plugin(direct_plugin.expect("已确认 Javadoc 插件声明"))
            } else if declared {
                (
                    "configured",
                    "plugin_declared_in_build_plugins",
                    "运行对应原生检查并读取诊断",
                )
            } else if managed_only
                || profile_only
                || inherited
                || possible_security
                || unresolved_direct_identity
            {
                (
                    "unknown",
                    "effective_model_or_profile_not_resolved",
                    "解析生效 Maven 模型以确定检查配置",
                )
            } else {
                (
                    "missing",
                    "plugin_not_declared",
                    "在项目中配置对应原生检查器或明确采用其他检查命令",
                )
            };
            entry(build_root, source, id, category, status, reason, action)
        })
        .collect()
}

fn inspect_javadoc_plugin(plugin: Node<'_, '_>) -> (&'static str, &'static str, &'static str) {
    let config = child(plugin, "configuration");
    let fail_on_error = config
        .and_then(|config| child(config, "failOnError"))
        .and_then(|value| value.text())
        .map(str::trim);
    match fail_on_error {
        Some("false") => {
            return (
                "invalid",
                "javadoc_errors_ignored",
                "设置 Maven Javadoc failOnError=true 后重新探测",
            );
        }
        Some("true") | None => {}
        Some(_) => {
            return (
                "unknown",
                "javadoc_error_policy_unresolved",
                "解析 Maven Javadoc 生效 failOnError 配置",
            );
        }
    }
    let doclint = config
        .and_then(|config| child(config, "doclint"))
        .and_then(|value| value.text())
        .map(str::trim);
    let Some(doclint) = doclint else {
        return (
            "configured",
            "plugin_declared_in_build_plugins",
            "运行对应原生检查并读取诊断",
        );
    };
    if doclint.is_empty() || doclint.contains("${") {
        return (
            "unknown",
            "javadoc_doclint_dynamic",
            "解析 Maven Javadoc 生效 doclint 配置",
        );
    }
    let mut missing_enabled = false;
    for token in doclint.split(',').map(str::trim) {
        match token {
            "all" | "missing" => missing_enabled = true,
            "none" | "-missing" => missing_enabled = false,
            "accessibility" | "html" | "reference" | "syntax" | "-accessibility" | "-html"
            | "-reference" | "-syntax" => {}
            _ => {
                return (
                    "unknown",
                    "javadoc_doclint_unresolved",
                    "核对 Javadoc doclint 表达式及实际启用的 missing 检查",
                );
            }
        }
    }
    if missing_enabled {
        (
            "configured",
            "plugin_declared_in_build_plugins",
            "运行对应原生检查并读取诊断",
        )
    } else {
        (
            "invalid",
            "javadoc_missing_check_disabled",
            "启用 Javadoc doclint missing 检查后重新探测",
        )
    }
}

fn inspect_p3c_plugin(plugin: Node<'_, '_>) -> (&'static str, &'static str, &'static str) {
    let config = child(plugin, "configuration");
    let dependency = child(plugin, "dependencies").and_then(|dependencies| {
        dependencies.children().find(|candidate| {
            named(*candidate, "dependency")
                && child(*candidate, "groupId")
                    .and_then(|value| value.text())
                    .is_some_and(|value| value.trim() == "com.alibaba.p3c")
                && child(*candidate, "artifactId")
                    .and_then(|value| value.text())
                    .is_some_and(|value| value.trim() == "p3c-pmd")
        })
    });
    let Some(dependency) = dependency else {
        let dynamic_dependency = child(plugin, "dependencies").is_some_and(|dependencies| {
            dependencies.children().any(|candidate| {
                named(candidate, "dependency")
                    && ["groupId", "artifactId"].iter().any(|field| {
                        child(candidate, field)
                            .and_then(|value| value.text())
                            .is_some_and(|value| value.contains("${"))
                    })
            })
        });
        if dynamic_dependency {
            return (
                "unknown",
                "p3c_dependency_identity_dynamic",
                "解析 Maven 生效插件依赖后确定是否使用 P3C",
            );
        }
        return (
            "missing",
            "p3c_artifact_not_declared",
            "在 Maven PMD 插件依赖中明确声明 P3C 制品",
        );
    };
    let skip = plugin_skip_value(plugin);
    if skip == Some("true") {
        return (
            "invalid",
            "p3c_plugin_explicitly_skipped",
            "移除 Maven PMD 的 skip=true 并重新探测",
        );
    }
    if skip.is_some_and(|value| value.contains("${")) {
        return (
            "unknown",
            "p3c_skip_value_dynamic",
            "解析 Maven 生效属性以确定是否跳过 P3C",
        );
    }
    let version = child(dependency, "version")
        .and_then(|value| value.text())
        .map(str::trim);
    if version != Some("2.1.1") {
        return (
            "unknown",
            "p3c_artifact_version_not_verified",
            "核对 P3C 制品版本与对应原生规则集",
        );
    }
    let rulesets = config
        .and_then(|config| child(config, "rulesets"))
        .map(|rulesets| {
            rulesets
                .children()
                .filter(|node| named(*node, "ruleset"))
                .filter_map(|node| node.text().map(str::trim))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    if rulesets.iter().any(|value| value.contains("${")) {
        return (
            "unknown",
            "p3c_ruleset_dynamic",
            "解析 Maven 生效规则集后重新探测",
        );
    }
    if rulesets.is_empty() {
        return (
            "missing",
            "p3c_ruleset_not_declared",
            "在 Maven PMD 中明确引用对应版本的阿里 P3C 规则集",
        );
    }
    if rulesets
        .iter()
        .any(|value| !P3C_2_1_1_RULESETS.contains(value))
        || rulesets.len()
            != rulesets
                .iter()
                .collect::<std::collections::BTreeSet<_>>()
                .len()
    {
        return (
            "unknown",
            "p3c_ruleset_selection_unresolved",
            "核对项目规则集；不能用隔离探针的其它规则代替项目配置",
        );
    }
    let error_policy = config
        .and_then(|config| child(config, "skipPmdError"))
        .and_then(|value| value.text())
        .map(str::trim);
    match error_policy {
        Some("false") => (
            "configured",
            "p3c_artifact_ruleset_and_error_policy_declared",
            "运行原生 Maven PMD 并核对 P3C 规则加载、目标覆盖和诊断",
        ),
        Some("true") => (
            "invalid",
            "p3c_processing_errors_skipped",
            "设置 skipPmdError=false，避免原生处理错误被静默跳过",
        ),
        _ => (
            "unknown",
            "p3c_processing_error_policy_unresolved",
            "核对 Maven PMD 的 skipPmdError 生效值",
        ),
    }
}

/// 仅从可静态确认的直接 POM 声明读取精确的 P3C 规则集；不解析继承或 profile。
pub fn configured_p3c_rulesets(bytes: &[u8]) -> Option<Vec<&'static str>> {
    let text = std::str::from_utf8(bytes).ok()?;
    let document = Document::parse_with_options(
        text,
        ParsingOptions {
            allow_dtd: false,
            nodes_limit: 100_000,
            entity_resolver: None,
        },
    )
    .ok()?;
    let root = document.root_element();
    if !named(root, "project")
        || child(root, "parent").is_some()
        || child(root, "profiles").is_some()
        || child(root, "build")
            .and_then(|build| child(build, "pluginManagement"))
            .is_some()
    {
        return None;
    }
    let plugins = child(root, "build").and_then(|build| child(build, "plugins"))?;
    let direct_plugins: Vec<_> = plugins
        .children()
        .filter(|node| plugin_identity(*node, "org.apache.maven.plugins", "maven-pmd-plugin"))
        .collect();
    if direct_plugins.len() != 1 {
        return None;
    }
    let plugin = direct_plugins[0];
    if inspect_p3c_plugin(plugin).0 != "configured"
        || child(plugin, "version")
            .and_then(|node| node.text())
            .map(str::trim)
            != Some("3.11.0")
    {
        return None;
    }
    let dependencies = child(plugin, "dependencies")?;
    let p3c_dependencies: Vec<_> = dependencies
        .children()
        .filter(|node| {
            named(*node, "dependency")
                && child(*node, "groupId")
                    .and_then(|value| value.text())
                    .map(str::trim)
                    == Some("com.alibaba.p3c")
                && child(*node, "artifactId")
                    .and_then(|value| value.text())
                    .map(str::trim)
                    == Some("p3c-pmd")
        })
        .collect();
    if p3c_dependencies.len() != 1 {
        return None;
    }
    for artifact in ["pmd-core", "pmd-java"] {
        let matches: Vec<_> = dependencies
            .children()
            .filter(|node| {
                named(*node, "dependency")
                    && child(*node, "groupId")
                        .and_then(|value| value.text())
                        .map(str::trim)
                        == Some("net.sourceforge.pmd")
                    && child(*node, "artifactId")
                        .and_then(|value| value.text())
                        .map(str::trim)
                        == Some(artifact)
            })
            .collect();
        if matches.len() != 1
            || child(matches[0], "version")
                .and_then(|value| value.text())
                .map(str::trim)
                != Some("6.15.0")
        {
            return None;
        }
    }
    let config = child(plugin, "configuration")?;
    if config.children().filter(Node::is_element).any(|node| {
        ![
            "rulesets",
            "skipPmdError",
            "linkXRef",
            "includeTests",
            "minimumPriority",
        ]
        .iter()
        .any(|name| named(node, name))
    }) || child(config, "minimumPriority")
        .and_then(|node| node.text())
        .map(str::trim)
        != Some("5")
    {
        return None;
    }
    let rulesets = child(config, "rulesets")?;
    let mut selected = Vec::new();
    for rule in rulesets.children().filter(Node::is_element) {
        if !named(rule, "ruleset") {
            return None;
        }
        let value = rule.text()?.trim();
        let canonical = P3C_2_1_1_RULESETS.iter().find(|known| **known == value)?;
        if selected.contains(canonical) {
            return None;
        }
        selected.push(*canonical);
    }
    (!selected.is_empty()).then_some(selected)
}

/// Gradle 构建脚本可能包含动态逻辑；静态发现不把未看到文本等同未配置。
pub fn inspect_gradle_unknown(build_root: &str, source: &str) -> Vec<CheckerConfiguration> {
    CHECKERS
        .iter()
        .map(|(id, category, _, _)| {
            let gradle_id = id.replace("java.maven.", "java.gradle.");
            entry(
                build_root,
                source,
                &gradle_id,
                category,
                "unknown",
                "gradle_model_not_resolved",
                "解析生效 Gradle 模型以确定检查配置",
            )
        })
        .collect()
}

/// 项目级 Ruff 配置文件缺失；不推断用户级配置或 CI 命令不存在。
pub fn inspect_ruff_missing(build_root: &str) -> CheckerConfiguration {
    entry(
        build_root,
        build_root,
        "python.ruff",
        "lint",
        "missing",
        "project_ruff_config_not_found",
        "核对项目 CI/钩子命令；需要时配置 Ruff lint 规则",
    )
}

/// 不能只靠项目文件确定 Ruff 的生效配置时保留未知。
pub fn inspect_ruff_unknown(build_root: &str, source: &str, reason: &str) -> CheckerConfiguration {
    entry(
        build_root,
        source,
        "python.ruff",
        "lint",
        "unknown",
        reason,
        "解析每个源码文件的 Ruff 生效配置后复查",
    )
}

/// 解析项目级 Ruff TOML；只确认明确 lint 设置，不把格式化配置算作 lint。
pub fn inspect_ruff_config(
    bytes: &[u8],
    build_root: &str,
    source: &str,
    pyproject: bool,
) -> CheckerConfiguration {
    let parsed = std::str::from_utf8(bytes)
        .ok()
        .and_then(|text| toml::from_str::<toml::Table>(text).ok());
    let Some(document) = parsed else {
        return if pyproject {
            inspect_ruff_unknown(build_root, source, "pyproject_toml_unparseable")
        } else {
            entry(
                build_root,
                source,
                "python.ruff",
                "lint",
                "invalid",
                "ruff_toml_invalid",
                "修正 Ruff TOML 语法后重新探测",
            )
        };
    };
    let configuration = if pyproject {
        match document
            .get("tool")
            .and_then(toml::Value::as_table)
            .and_then(|tool| tool.get("ruff"))
        {
            Some(toml::Value::Table(configuration)) => configuration,
            Some(_) => {
                return entry(
                    build_root,
                    source,
                    "python.ruff",
                    "lint",
                    "invalid",
                    "ruff_section_invalid",
                    "修正 tool.ruff 配置结构后重新探测",
                );
            }
            None => {
                return entry(
                    build_root,
                    source,
                    "python.ruff",
                    "lint",
                    "missing",
                    "ruff_section_not_declared",
                    "核对项目 CI/钩子命令；需要时配置 Ruff lint 规则",
                );
            }
        }
    } else {
        &document
    };
    if configuration.contains_key("extend") {
        return inspect_ruff_unknown(build_root, source, "ruff_extend_not_resolved");
    }
    if configuration
        .get("lint")
        .is_some_and(|value| value.as_table().is_none())
    {
        return entry(
            build_root,
            source,
            "python.ruff",
            "lint",
            "invalid",
            "ruff_lint_section_invalid",
            "修正 Ruff lint 配置结构后重新探测",
        );
    }
    let has_lint_table = configuration.contains_key("lint");
    let has_lint_setting = ["select", "ignore", "extend-select", "extend-ignore", "fix"]
        .iter()
        .any(|name| configuration.contains_key(*name));
    if has_lint_table || has_lint_setting {
        entry(
            build_root,
            source,
            "python.ruff",
            "lint",
            "configured",
            "explicit_ruff_lint_configuration",
            "用原生 Ruff 核对生效规则、扫描范围和诊断",
        )
    } else {
        inspect_ruff_unknown(build_root, source, "ruff_lint_intent_not_explicit")
    }
}

fn entry(
    root: &str,
    source: &str,
    id: &str,
    category: &str,
    status: &str,
    reason: &str,
    action: &str,
) -> CheckerConfiguration {
    CheckerConfiguration {
        build_root: root.into(),
        checker_id: id.into(),
        category: category.into(),
        configuration: status.into(),
        configuration_ref: source.into(),
        reason: reason.into(),
        next_action: action.into(),
    }
}

fn named(node: Node<'_, '_>, name: &str) -> bool {
    node.is_element() && node.tag_name().name() == name
}

fn child<'a, 'input>(node: Node<'a, 'input>, name: &str) -> Option<Node<'a, 'input>> {
    node.children().find(|child| named(*child, name))
}

fn find_plugin<'a, 'input>(
    plugins: Node<'a, 'input>,
    group: &str,
    artifact: &str,
) -> Option<Node<'a, 'input>> {
    plugins
        .children()
        .find(|node| named(*node, "plugin") && plugin_identity(*node, group, artifact))
}

fn plugin_identity(node: Node<'_, '_>, group: &str, artifact: &str) -> bool {
    let artifact_matches = child(node, "artifactId")
        .and_then(|value| value.text())
        .is_some_and(|value| value.trim() == artifact);
    let group_value = child(node, "groupId")
        .and_then(|value| value.text())
        .map(str::trim)
        .unwrap_or("org.apache.maven.plugins");
    artifact_matches && group_value == group
}

fn has_findsecbugs_dependency(plugin: Node<'_, '_>) -> bool {
    child(plugin, "dependencies").is_some_and(|dependencies| {
        dependencies.children().any(|dependency| {
            named(dependency, "dependency")
                && child(dependency, "groupId")
                    .and_then(|value| value.text())
                    .is_some_and(|value| value.trim() == "com.h3xstream.findsecbugs")
                && child(dependency, "artifactId")
                    .and_then(|value| value.text())
                    .is_some_and(|value| value.trim() == "findsecbugs-plugin")
        })
    })
}

fn plugin_skip_value<'a, 'input>(plugin: Node<'a, 'input>) -> Option<&'a str> {
    child(plugin, "configuration")
        .and_then(|configuration| child(configuration, "skip"))
        .and_then(|value| value.text())
        .map(str::trim)
}

fn has_unresolved_plugin_identity(plugins: Node<'_, '_>) -> bool {
    plugins
        .children()
        .filter(|node| named(*node, "plugin"))
        .any(|plugin| {
            ["groupId", "artifactId"].iter().any(|field| {
                child(plugin, field)
                    .and_then(|value| value.text())
                    .is_some_and(|value| value.contains("${"))
            })
        })
}
