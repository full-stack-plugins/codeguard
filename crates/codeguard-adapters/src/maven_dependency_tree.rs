//! Maven Dependency Plugin JSON 依赖树的有界解析；不把图解析当成漏洞结论。

use serde::Deserialize;

/// 原生依赖树中的一个已解析组件；根项目也占一个节点。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MavenDependencyNode {
    pub group_id: String,
    pub artifact_id: String,
    pub version: String,
    pub artifact_type: String,
    pub scope: String,
    pub classifier: String,
    pub optional: bool,
}

/// 按原生父子关系保留的依赖树；边使用节点下标，未做去重或漏洞推断。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MavenDependencyTree {
    pub nodes: Vec<MavenDependencyNode>,
    pub edges: Vec<(usize, usize)>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct NativeNode {
    group_id: String,
    artifact_id: String,
    version: String,
    #[serde(rename = "type")]
    artifact_type: String,
    scope: String,
    classifier: String,
    optional: String,
    #[serde(default)]
    children: Vec<NativeNode>,
}

/// 解析 Maven Dependency Plugin 3.7+ 的 JSON 输出。
/// 参数为本次原生命令产出的报告字节；返回精确父子图或结构错误。
pub fn parse_maven_dependency_tree_json(bytes: &[u8]) -> Result<MavenDependencyTree, &'static str> {
    if bytes.is_empty() || bytes.len() > 4 * 1024 * 1024 {
        return Err("dependency_tree_size_invalid");
    }
    let native: NativeNode =
        serde_json::from_slice(bytes).map_err(|_| "dependency_tree_json_invalid")?;
    let mut tree = MavenDependencyTree {
        nodes: Vec::new(),
        edges: Vec::new(),
    };
    append_node(native, None, 0, &mut tree)?;
    Ok(tree)
}

fn append_node(
    native: NativeNode,
    parent: Option<usize>,
    depth: usize,
    tree: &mut MavenDependencyTree,
) -> Result<(), &'static str> {
    if depth > 64 || tree.nodes.len() >= 10_000 {
        return Err("dependency_tree_complexity_exceeded");
    }
    if !coordinate(&native.group_id)
        || !coordinate(&native.artifact_id)
        || !coordinate(&native.version)
        || !coordinate(&native.artifact_type)
        || (!native.classifier.is_empty() && !coordinate(&native.classifier))
        || !matches!(native.optional.as_str(), "true" | "false")
        || !matches!(
            native.scope.as_str(),
            "" | "compile" | "provided" | "runtime" | "test" | "system" | "import"
        )
        || (parent.is_some() && native.scope.is_empty())
        || (parent.is_none() && !native.scope.is_empty())
    {
        return Err("dependency_tree_component_invalid");
    }
    let index = tree.nodes.len();
    tree.nodes.push(MavenDependencyNode {
        group_id: native.group_id,
        artifact_id: native.artifact_id,
        version: native.version,
        artifact_type: native.artifact_type,
        scope: native.scope,
        classifier: native.classifier,
        optional: native.optional == "true",
    });
    if let Some(parent) = parent {
        tree.edges.push((parent, index));
    }
    for child in native.children {
        append_node(child, Some(index), depth + 1, tree)?;
    }
    Ok(())
}

fn coordinate(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 200
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-' | b'+'))
}
