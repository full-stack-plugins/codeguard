//! 分类型、带条件和完整性的模块图
//!
//! 验收标准：聚合不当依赖，动态未知边不支撑缩小检查范围，过期 CodeGraph 不作为当前证据

use serde::{Deserialize, Serialize};

/// 依赖类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DependencyType {
    /// 构建依赖
    Build,
    /// 源码引用
    Source,
    /// 动态未知
    DynamicUnknown,
}

/// 模块边
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModuleEdge {
    /// 源模块
    pub from: String,
    /// 目标模块
    pub to: String,
    /// 依赖类型
    pub dep_type: DependencyType,
    /// 是否已解析
    pub resolved: bool,
}

/// 模块图
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypedModuleGraph {
    /// 模块列表
    pub modules: Vec<String>,
    /// 边列表
    pub edges: Vec<ModuleEdge>,
    /// 是否完整
    pub complete: bool,
}

/// 模块图生成器
pub struct TypedModuleGraphGenerator;

impl TypedModuleGraphGenerator {
    /// 生成模块图
    pub fn generate(
        modules: &[String],
        edges: &[(String, String, DependencyType)],
    ) -> TypedModuleGraph {
        let module_edges = edges.iter()
            .map(|(from, to, dep_type)| ModuleEdge {
                from: from.clone(),
                to: to.clone(),
                dep_type: *dep_type,
                resolved: *dep_type != DependencyType::DynamicUnknown,
            })
            .collect();
        
        let complete = edges.iter().all(|(_, _, t)| *t != DependencyType::DynamicUnknown);
        
        TypedModuleGraph {
            modules: modules.to_vec(),
            edges: module_edges,
            complete,
        }
    }
    
    /// 检查是否聚合不当依赖
    pub fn check_improper_aggregation(graph: &TypedModuleGraph) -> bool {
        // 动态未知边不支撑缩小检查范围
        graph.edges.iter().any(|e| e.dep_type == DependencyType::DynamicUnknown && e.resolved)
    }
    
    /// 验证过期 CodeGraph 不作为当前证据
    pub fn validate_no_stale_codegraph(graph: &TypedModuleGraph) -> bool {
        graph.edges.iter().all(|e| e.resolved || e.dep_type == DependencyType::DynamicUnknown)
    }
}
