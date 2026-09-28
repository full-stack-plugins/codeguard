//! Maven POM 的有界静态模块声明；不执行有效模型解析或属性插值。

use std::collections::{BTreeMap, BTreeSet};

/// 一份 POM 直接声明的坐标、聚合模块和依赖；未知条件单独保留。
#[derive(Clone, Debug)]
pub struct MavenModuleModel {
    /// 仅在三个字段均直接声明且无变量时存在。
    pub coordinates: Option<(String, String, String)>,
    /// 项目直接 version 字段；不从父/profile 或变量恢复生效版本。
    pub package_version: Option<String>,
    /// 直接 modules 声明，不代表依赖关系。
    pub modules: Vec<String>,
    /// 直接依赖的 group/artifact/version/scope 声明。
    pub dependencies: Vec<(String, String, String, String)>,
    /// 父模型、profile、管理或未解析字段等缺口。
    pub unresolved: BTreeSet<String>,
    /// 仅直接声明的语言目标/方言；不是本机版本或有效构建模型。
    pub language_targets: BTreeMap<String, String>,
}

impl MavenModuleModel {
    /// 从至多 256 KiB 原始 POM 字节观察直接声明；坏 XML/命名空间返回错误。
    /// 返回内容不代表 Maven 生效模型，也不证明依赖已解析或检查覆盖完整。
    pub fn observe(bytes: &[u8]) -> Result<Self, &'static str> {
        if bytes.len() > 256 * 1024 {
            return Err("maven_module_manifest_limit_exceeded");
        }
        let text = std::str::from_utf8(bytes).map_err(|_| "maven_module_manifest_not_utf8")?;
        let document =
            roxmltree::Document::parse(text).map_err(|_| "maven_module_manifest_invalid_xml")?;
        let root = document.root_element();
        if root.tag_name().name() != "project"
            || !matches!(
                root.tag_name().namespace(),
                None | Some("http://maven.apache.org/POM/4.0.0")
            )
        {
            return Err("maven_module_manifest_invalid_root");
        }
        let mut result = Self {
            coordinates: coordinates(root),
            package_version: field(root, "version")
                .filter(|value| valid_package_version(value))
                .map(str::to_owned),
            modules: Vec::new(),
            dependencies: Vec::new(),
            unresolved: BTreeSet::new(),
            language_targets: BTreeMap::new(),
        };
        if result.package_version.is_none() {
            result.unresolved.insert(
                if children(root, "version").next().is_some() {
                    "package_version_unresolved"
                } else {
                    "package_version_not_declared"
                }
                .into(),
            );
        }
        let properties: Vec<_> = children(root, "properties").collect();
        if properties.len() > 1 {
            result
                .unresolved
                .insert("language_target_properties_unresolved".into());
        } else if let Some(properties) = properties.first().copied() {
            for key in [
                "maven.compiler.release",
                "maven.compiler.source",
                "maven.compiler.target",
            ] {
                if children(properties, key).next().is_none() {
                    continue;
                }
                let value = field(properties, key).filter(|value| valid_java_target(value, key));
                if let Some(value) = value {
                    result.language_targets.insert(key.into(), value.into());
                } else {
                    result
                        .unresolved
                        .insert(format!("language_target_unresolved:{key}"));
                }
            }
        }
        if result.coordinates.is_none() {
            result
                .unresolved
                .insert("project_coordinates_unresolved".into());
        }
        for tag in ["parent", "profiles", "dependencyManagement", "properties"] {
            if children(root, tag).next().is_some() {
                result.unresolved.insert(format!("{tag}_not_evaluated"));
            }
        }
        for modules in children(root, "modules") {
            for module in children(modules, "module") {
                if let Some(value) = scalar(module) {
                    result.modules.push(value.into());
                } else {
                    result
                        .unresolved
                        .insert("module_declaration_unresolved".into());
                }
            }
        }
        for dependencies in children(root, "dependencies") {
            for dependency in children(dependencies, "dependency") {
                let Some((group, artifact, version)) = coordinates(dependency) else {
                    result
                        .unresolved
                        .insert("dependency_coordinates_unresolved".into());
                    continue;
                };
                let scope = if children(dependency, "scope").next().is_none() {
                    Some("compile")
                } else {
                    field(dependency, "scope")
                };
                if !scope.is_some_and(|value| {
                    matches!(
                        value,
                        "compile" | "provided" | "runtime" | "test" | "system" | "import"
                    )
                }) || children(dependency, "optional").next().is_some()
                    || children(dependency, "type").next().is_some()
                    || children(dependency, "classifier").next().is_some()
                    || children(dependency, "systemPath").next().is_some()
                {
                    result
                        .unresolved
                        .insert("dependency_attributes_unresolved".into());
                    continue;
                }
                result
                    .dependencies
                    .push((group, artifact, version, scope.unwrap().into()));
            }
        }
        Ok(result)
    }
}

fn children<'a>(
    node: roxmltree::Node<'a, 'a>,
    name: &'a str,
) -> impl Iterator<Item = roxmltree::Node<'a, 'a>> {
    node.children().filter(move |child| {
        child.is_element()
            && child.tag_name().name() == name
            && child.tag_name().namespace() == node.tag_name().namespace()
    })
}
fn scalar<'a>(node: roxmltree::Node<'a, 'a>) -> Option<&'a str> {
    // 注释切分的多段文本不能只取首段，避免把另一个模块误认作本地目标。
    if node.children().any(|child| child.is_element())
        || node.children().filter(|child| child.is_text()).count() != 1
    {
        return None;
    }
    let value = node.text()?.trim();
    (!value.is_empty() && !value.contains(['$', '{', '}']) && !value.chars().any(char::is_control))
        .then_some(value)
}
fn field<'a>(node: roxmltree::Node<'a, 'a>, name: &'a str) -> Option<&'a str> {
    let mut matching = children(node, name);
    let value = scalar(matching.next()?)?;
    matching.next().is_none().then_some(value)
}
fn coordinates(node: roxmltree::Node<'_, '_>) -> Option<(String, String, String)> {
    let group = field(node, "groupId")?;
    let artifact = field(node, "artifactId")?;
    let version = field(node, "version")?;
    if ![group, artifact, version].into_iter().all(|value| {
        value.len() <= 1024
            && value
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'-' | b'_'))
    }) {
        return None;
    }
    Some((group.into(), artifact.into(), version.into()))
}

fn valid_java_target(value: &str, key: &str) -> bool {
    if key != "maven.compiler.release"
        && matches!(
            value,
            "1.1" | "1.2" | "1.3" | "1.4" | "1.5" | "1.6" | "1.7" | "1.8"
        )
    {
        return true;
    }
    !value.is_empty()
        && value.bytes().all(|byte| byte.is_ascii_digit())
        && value.parse::<u16>().is_ok_and(|number| number > 0)
}

fn valid_package_version(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 256
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'-' | b'_' | b'+'))
}
