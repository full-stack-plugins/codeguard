//! OWASP Maven 检查器私有重放前的静态 POM 资格解析；不宣称生效模型完整。

use std::collections::BTreeSet;

use roxmltree::{Document, Node, ParsingOptions};

/// 能直接重放的项目和原生插件版本；动态配置需要另行解析。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OwaspMavenPomPlan {
    pub group_id: String,
    pub artifact_id: String,
    pub version: String,
    pub plugin_version: String,
}

/// 只接受直接声明、固定版本、无父 POM/profile/额外插件与动态配置的简单 Maven 项目。
/// 返回值只表示可尝试原生命令；不表示数据库新鲜或漏洞检查完成。
pub fn owasp_maven_pom_plan(bytes: &[u8]) -> Option<OwaspMavenPomPlan> {
    if bytes.is_empty() || bytes.len() > 4 * 1024 * 1024 {
        return None;
    }
    let xml = std::str::from_utf8(bytes).ok()?;
    if xml.contains("${") || xml.contains("<!DOCTYPE") {
        return None;
    }
    let document = Document::parse_with_options(
        xml,
        ParsingOptions {
            allow_dtd: false,
            nodes_limit: 30_000,
            entity_resolver: None,
        },
    )
    .ok()?;
    let root = document.root_element();
    let namespace = root.tag_name().namespace();
    if root.tag_name().name() != "project"
        || !matches!(namespace, None | Some("http://maven.apache.org/POM/4.0.0"))
        || !shape(
            root,
            namespace,
            &[
                "modelVersion",
                "groupId",
                "artifactId",
                "version",
                "packaging",
                "build",
                "dependencies",
            ],
        )
        || child(root, "modelVersion").and_then(scalar).as_deref() != Some("4.0.0")
        || child(root, "packaging").is_some_and(|node| scalar(node).as_deref() != Some("jar"))
    {
        return None;
    }
    let group_id = scalar(child(root, "groupId")?)?;
    let artifact_id = scalar(child(root, "artifactId")?)?;
    let version = scalar(child(root, "version")?)?;
    if !coordinate(&group_id) || !coordinate(&artifact_id) || !fixed_version(&version) {
        return None;
    }
    let build = child(root, "build")?;
    if !shape(build, namespace, &["plugins"]) {
        return None;
    }
    let plugins = child(build, "plugins")?;
    if plugins.attributes().len() != 0 {
        return None;
    }
    if plugins
        .children()
        .any(|part| part.is_text() && !part.text().unwrap_or("").trim().is_empty())
    {
        return None;
    }
    let mut plugin_version = None;
    let mut dependency_seen = false;
    for plugin in plugins.children().filter(Node::is_element) {
        if plugin.tag_name().name() != "plugin"
            || plugin.tag_name().namespace() != namespace
            || !shape(plugin, namespace, &["groupId", "artifactId", "version"])
        {
            return None;
        }
        let group = child(plugin, "groupId").and_then(scalar);
        let artifact = child(plugin, "artifactId").and_then(scalar);
        let version = child(plugin, "version").and_then(scalar);
        match (group.as_deref(), artifact.as_deref(), version.as_deref()) {
            (Some("org.owasp"), Some("dependency-check-maven"), Some(value))
                if plugin_version.is_none() && fixed_version(value) =>
            {
                plugin_version = version
            }
            (
                None | Some("org.apache.maven.plugins"),
                Some("maven-dependency-plugin"),
                Some("3.8.1"),
            ) if !dependency_seen => dependency_seen = true,
            _ => return None,
        }
    }
    let plugin_version = plugin_version?;
    if let Some(dependencies) = child(root, "dependencies") {
        if dependencies.attributes().len() != 0
            || !dependencies.children().all(|entry| {
                if !entry.is_element() {
                    return !entry.is_text() || entry.text().unwrap_or("").trim().is_empty();
                }
                entry.tag_name().namespace() == namespace
                    && entry.tag_name().name() == "dependency"
                    && shape(
                        entry,
                        namespace,
                        &[
                            "groupId",
                            "artifactId",
                            "version",
                            "scope",
                            "type",
                            "classifier",
                            "optional",
                        ],
                    )
                    && child(entry, "groupId")
                        .and_then(scalar)
                        .is_some_and(|value| coordinate(&value))
                    && child(entry, "artifactId")
                        .and_then(scalar)
                        .is_some_and(|value| coordinate(&value))
                    && child(entry, "version")
                        .and_then(scalar)
                        .is_some_and(|value| fixed_version(&value))
                    && ["type", "classifier"].iter().all(|name| {
                        child(entry, name)
                            .is_none_or(|node| scalar(node).is_some_and(|value| coordinate(&value)))
                    })
                    && child(entry, "scope").is_none_or(|node| {
                        matches!(
                            scalar(node).as_deref(),
                            Some("compile" | "provided" | "runtime" | "test")
                        )
                    })
                    && child(entry, "optional").is_none_or(|node| {
                        matches!(scalar(node).as_deref(), Some("true" | "false"))
                    })
            })
        {
            return None;
        }
    }
    Some(OwaspMavenPomPlan {
        group_id,
        artifact_id,
        version,
        plugin_version,
    })
}

fn shape(node: Node<'_, '_>, namespace: Option<&str>, allowed: &[&str]) -> bool {
    if node.attributes().len() != 0 {
        return false;
    }
    let mut seen = BTreeSet::new();
    node.children().all(|part| {
        if part.is_element() {
            part.tag_name().namespace() == namespace
                && allowed.contains(&part.tag_name().name())
                && seen.insert(part.tag_name().name())
        } else {
            !part.is_text() || part.text().unwrap_or("").trim().is_empty()
        }
    })
}

fn child<'a, 'input>(node: Node<'a, 'input>, name: &str) -> Option<Node<'a, 'input>> {
    node.children()
        .find(|part| part.is_element() && part.tag_name().name() == name)
}

fn scalar(node: Node<'_, '_>) -> Option<String> {
    if node.attributes().len() != 0 || node.children().any(|part| !part.is_text()) {
        return None;
    }
    Some(node.text()?.trim().to_owned())
}

fn coordinate(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 200
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-' | b'+'))
}

fn fixed_version(value: &str) -> bool {
    coordinate(value) && !matches!(value, "LATEST" | "RELEASE") && !value.ends_with("-SNAPSHOT")
}
