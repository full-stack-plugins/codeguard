//! Maven 依赖树私有重放的静态 POM 范围门禁；不替代生效模型解析。

use roxmltree::{Document, Node, ParsingOptions};
use std::collections::BTreeSet;

/// 只允许无继承、无扩展、无额外插件的静态依赖项目进入私有原生探针。
/// 参数为原 POM 字节；返回仅代表重放资格，不证明依赖图或项目覆盖完整。
pub fn dependency_pom_direct_replay_eligible(bytes: &[u8]) -> bool {
    let Ok(xml) = std::str::from_utf8(bytes) else {
        return false;
    };
    if xml.len() > 4 * 1024 * 1024 || xml.contains("${") || xml.contains("<!DOCTYPE") {
        return false;
    }
    let Ok(document) = Document::parse_with_options(
        xml,
        ParsingOptions {
            allow_dtd: false,
            nodes_limit: 30_000,
            entity_resolver: None,
        },
    ) else {
        return false;
    };
    let root = document.root_element();
    let ns = root.tag_name().namespace();
    if root.tag_name().name() != "project"
        || !matches!(ns, None | Some("http://maven.apache.org/POM/4.0.0"))
        || !shape(
            root,
            ns,
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
        || !child(root, "groupId")
            .and_then(scalar)
            .is_some_and(|s| coordinate(&s))
        || !child(root, "artifactId")
            .and_then(scalar)
            .is_some_and(|s| coordinate(&s))
        || !child(root, "version")
            .and_then(scalar)
            .is_some_and(|s| coordinate(&s))
        || child(root, "packaging").is_some_and(|node| scalar(node).as_deref() != Some("jar"))
    {
        return false;
    }
    let Some(build) = child(root, "build") else {
        return false;
    };
    if !shape(build, ns, &["plugins"]) {
        return false;
    }
    let Some(plugins) = child(build, "plugins") else {
        return false;
    };
    if plugins.attributes().len() != 0 {
        return false;
    }
    if plugins
        .children()
        .any(|part| part.is_text() && !part.text().unwrap_or("").trim().is_empty())
    {
        return false;
    }
    let mut dependency_seen = false;
    let mut owasp_seen = false;
    for plugin in plugins.children().filter(Node::is_element) {
        if plugin.tag_name().name() != "plugin"
            || plugin.tag_name().namespace() != ns
            || !shape(plugin, ns, &["groupId", "artifactId", "version"])
        {
            return false;
        }
        let group = child(plugin, "groupId").and_then(scalar);
        let artifact = child(plugin, "artifactId").and_then(scalar);
        let version = child(plugin, "version").and_then(scalar);
        match (group.as_deref(), artifact.as_deref(), version.as_deref()) {
            (
                None | Some("org.apache.maven.plugins"),
                Some("maven-dependency-plugin"),
                Some("3.8.1"),
            ) if !dependency_seen => dependency_seen = true,
            (Some("org.owasp"), Some("dependency-check-maven"), Some(value))
                if !owasp_seen && pinned_version(value) =>
            {
                owasp_seen = true
            }
            _ => return false,
        }
    }
    if !dependency_seen {
        return false;
    }
    let Some(dependencies) = child(root, "dependencies") else {
        return true;
    };
    if dependencies.attributes().len() != 0 {
        return false;
    }
    dependencies.children().all(|entry| {
        if !entry.is_element() {
            return !entry.is_text() || entry.text().unwrap_or("").trim().is_empty();
        }
        entry.tag_name().namespace() == ns
            && entry.tag_name().name() == "dependency"
            && shape(
                entry,
                ns,
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
            && ["groupId", "artifactId", "version"].iter().all(|name| {
                child(entry, name)
                    .and_then(scalar)
                    .is_some_and(|s| coordinate(&s))
            })
            && ["type", "classifier"].iter().all(|name| {
                child(entry, name).is_none_or(|node| scalar(node).is_some_and(|s| coordinate(&s)))
            })
            && child(entry, "scope").is_none_or(|node| {
                matches!(
                    scalar(node).as_deref(),
                    Some("compile" | "provided" | "runtime" | "test")
                )
            })
            && child(entry, "optional")
                .is_none_or(|node| matches!(scalar(node).as_deref(), Some("true" | "false")))
    })
}

fn pinned_version(value: &str) -> bool {
    coordinate(value) && !matches!(value, "LATEST" | "RELEASE") && !value.ends_with("-SNAPSHOT")
}

/// 从已确认可重放的原 POM 取得根项目坐标，用于核对原生图归属。
/// 参数为原 POM 字节；不合资格或坐标不可读时返回 None。
pub fn dependency_pom_project_identity(bytes: &[u8]) -> Option<(String, String, String)> {
    if !dependency_pom_direct_replay_eligible(bytes) {
        return None;
    }
    let xml = std::str::from_utf8(bytes).ok()?;
    let document = Document::parse(xml).ok()?;
    let root = document.root_element();
    Some((
        scalar(child(root, "groupId")?)?,
        scalar(child(root, "artifactId")?)?,
        scalar(child(root, "version")?)?,
    ))
}

fn shape(node: Node<'_, '_>, ns: Option<&str>, allowed: &[&str]) -> bool {
    if node.attributes().len() != 0 {
        return false;
    }
    let mut seen = BTreeSet::new();
    node.children().all(|part| {
        if part.is_element() {
            part.tag_name().namespace() == ns
                && allowed.contains(&part.tag_name().name())
                && seen.insert(part.tag_name().name())
        } else {
            !part.is_text() || part.text().unwrap_or("").trim().is_empty()
        }
    })
}

fn child<'a, 'input>(node: Node<'a, 'input>, name: &str) -> Option<Node<'a, 'input>> {
    node.children()
        .find(|entry| entry.is_element() && entry.tag_name().name() == name)
}

fn scalar(node: Node<'_, '_>) -> Option<String> {
    if node.attributes().len() != 0 || node.children().any(|entry| !entry.is_text()) {
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
