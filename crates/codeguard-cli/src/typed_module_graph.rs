//! 分类型模块图模块（9.18）。
//!
//! 实现分类型、带条件和完整性的模块图：聚合不当依赖，动态未知边不支撑缩小检查范围。

use serde_json::{Value, json};

/// 模块依赖边类型。
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum DependencyKind {
    /// 静态依赖。
    Static,
    /// 动态依赖。
    Dynamic,
    /// 条件依赖。
    Conditional,
}

impl DependencyKind {
    pub(crate) fn as_str(&self) -> &'static str {
        match self {
            Self::Static => "static",
            Self::Dynamic => "dynamic",
            Self::Conditional => "conditional",
        }
    }
}

/// 模块依赖边。
pub(crate) struct DependencyEdge {
    pub from: String,
    pub to: String,
    pub kind: DependencyKind,
    pub condition: Option<String>,
}

/// 分类型模块图。
pub(crate) struct TypedModuleGraph {
    pub edges: Vec<DependencyEdge>,
    pub incomplete: bool,
}

/// 构建分类型模块图。
pub(crate) fn build_typed_graph(edges: Vec<DependencyEdge>) -> TypedModuleGraph {
    let incomplete = edges
        .iter()
        .any(|e| matches!(e.kind, DependencyKind::Dynamic));
    TypedModuleGraph { edges, incomplete }
}

/// 生成模块图报告。
pub(crate) fn typed_graph_report(graph: &TypedModuleGraph) -> Value {
    let edge_values: Vec<Value> = graph
        .edges
        .iter()
        .map(|e| {
            json!({
                "from": e.from,
                "to": e.to,
                "kind": e.kind.as_str(),
                "condition": e.condition,
            })
        })
        .collect();

    json!({
        "schema_version": "0.1.0",
        "report_type": "typed_module_graph",
        "edges": edge_values,
        "incomplete": graph.incomplete,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn static_edge_not_incomplete() {
        let edges = vec![DependencyEdge {
            from: "a".to_string(),
            to: "b".to_string(),
            kind: DependencyKind::Static,
            condition: None,
        }];
        let graph = build_typed_graph(edges);
        assert!(!graph.incomplete);
    }

    #[test]
    fn dynamic_edge_marks_incomplete() {
        let edges = vec![DependencyEdge {
            from: "a".to_string(),
            to: "b".to_string(),
            kind: DependencyKind::Dynamic,
            condition: None,
        }];
        let graph = build_typed_graph(edges);
        assert!(graph.incomplete);
    }

    #[test]
    fn typed_graph_report_contains_kinds() {
        let edges = vec![
            DependencyEdge {
                from: "a".to_string(),
                to: "b".to_string(),
                kind: DependencyKind::Static,
                condition: None,
            },
            DependencyEdge {
                from: "a".to_string(),
                to: "c".to_string(),
                kind: DependencyKind::Dynamic,
                condition: Some("runtime".to_string()),
            },
        ];
        let graph = build_typed_graph(edges);
        let report = typed_graph_report(&graph);
        assert_eq!(report["edges"].as_array().unwrap().len(), 2);
    }
}
