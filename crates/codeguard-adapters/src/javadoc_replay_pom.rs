//! Maven Javadoc 原 POM 私有重放的保守资格检查；不能证明生效模型或完整源集。

use roxmltree::{Document, Node, ParsingOptions};
use std::collections::BTreeSet;

/// 仅允许一个无继承、无扩展、无其它插件的静态 Javadoc 3.12.0 项目。
///
/// 通过只代表可以将原 POM 与显式源码放进私有快照作局部原生观察，
/// 不代表项目依赖、生成源码、生命周期和交付质量已经核验。
pub fn javadoc_pom_direct_replay_eligible(bytes: &[u8]) -> bool {
    let Ok(xml) = std::str::from_utf8(bytes) else {
        return false;
    };
    if xml.contains("${") || xml.contains("<!DOCTYPE") {
        return false;
    }
    let Ok(document) = Document::parse_with_options(
        xml,
        ParsingOptions {
            allow_dtd: false,
            nodes_limit: 20_000,
            entity_resolver: None,
        },
    ) else {
        return false;
    };
    let project = document.root_element();
    let namespace = project.tag_name().namespace();
    if project.tag_name().name() != "project"
        || !matches!(namespace, None | Some("http://maven.apache.org/POM/4.0.0"))
        || !shape(
            project,
            namespace,
            &[
                "modelVersion",
                "groupId",
                "artifactId",
                "version",
                "packaging",
                "build",
            ],
        )
    {
        return false;
    }
    let (Some(model), Some(group), Some(artifact), Some(version), Some(build)) = (
        child(project, "modelVersion"),
        child(project, "groupId"),
        child(project, "artifactId"),
        child(project, "version"),
        child(project, "build"),
    ) else {
        return false;
    };
    if scalar(model).as_deref() != Some("4.0.0")
        || !scalar(group).is_some_and(|value| safe_coordinate(&value))
        || !scalar(artifact).is_some_and(|value| safe_coordinate(&value))
        || !scalar(version).is_some_and(|value| safe_coordinate(&value))
        || child(project, "packaging").is_some_and(|node| scalar(node).as_deref() != Some("jar"))
        || !shape(build, namespace, &["plugins"])
    {
        return false;
    }
    let Some(plugins) = child(build, "plugins") else {
        return false;
    };
    if !shape(plugins, namespace, &["plugin"]) {
        return false;
    }
    let mut declared = plugins.children().filter(Node::is_element);
    let (Some(plugin), None) = (declared.next(), declared.next()) else {
        return false;
    };
    if !shape(
        plugin,
        namespace,
        &["groupId", "artifactId", "version", "configuration"],
    ) || child(plugin, "groupId")
        .is_some_and(|node| scalar(node).as_deref() != Some("org.apache.maven.plugins"))
        || child(plugin, "artifactId").and_then(scalar).as_deref() != Some("maven-javadoc-plugin")
        || child(plugin, "version").and_then(scalar).as_deref() != Some("3.12.0")
    {
        return false;
    }
    let Some(configuration) = child(plugin, "configuration") else {
        return false;
    };
    shape(
        configuration,
        namespace,
        &["doclint", "failOnWarnings", "failOnError", "encoding"],
    ) && child(configuration, "doclint").and_then(scalar).as_deref() == Some("missing")
        && child(configuration, "failOnWarnings")
            .is_none_or(|node| matches!(scalar(node).as_deref(), Some("true" | "false")))
        && child(configuration, "failOnError")
            .is_none_or(|node| scalar(node).as_deref() == Some("true"))
        && child(configuration, "encoding")
            .is_none_or(|node| scalar(node).as_deref() == Some("UTF-8"))
}

fn shape(node: Node<'_, '_>, namespace: Option<&str>, allowed: &[&str]) -> bool {
    if node.attributes().len() != 0 {
        return false;
    }
    let mut seen = BTreeSet::new();
    for child in node.children() {
        if child.is_element() {
            if child.tag_name().namespace() != namespace
                || !allowed.contains(&child.tag_name().name())
                || !seen.insert(child.tag_name().name())
            {
                return false;
            }
        } else if child.is_text() && !child.text().unwrap_or("").trim().is_empty() {
            return false;
        }
    }
    true
}

fn child<'a, 'input>(node: Node<'a, 'input>, name: &str) -> Option<Node<'a, 'input>> {
    node.children()
        .find(|entry| entry.is_element() && entry.tag_name().name() == name)
}

fn scalar(node: Node<'_, '_>) -> Option<String> {
    if node.attributes().len() != 0 || node.children().any(|child| !child.is_text()) {
        return None;
    }
    Some(node.text()?.trim().to_owned())
}

fn safe_coordinate(value: &str) -> bool {
    !value.is_empty()
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
}
