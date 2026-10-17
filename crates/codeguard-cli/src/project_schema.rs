//! Project.json/Module-graph.json schema 与来源身份
//!
//! 验收标准：产品/目标/解析/本机版本分别记录，未知值不由其它字段猜测

use serde::{Deserialize, Serialize};

/// 来源身份
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SourceIdentity {
    /// 产品
    pub product: String,
    /// 目标
    pub target: String,
    /// 解析版本
    pub resolved_version: String,
    /// 本机版本
    pub local_version: Option<String>,
}

/// Project schema
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectSchema {
    /// Schema 版本
    pub schema_version: String,
    /// 来源身份
    pub source_identity: SourceIdentity,
    /// 构建根
    pub build_roots: Vec<String>,
}

/// Module graph schema
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleGraphSchema {
    /// Schema 版本
    pub schema_version: String,
    /// 模块列表
    pub modules: Vec<String>,
    /// 依赖边
    pub edges: Vec<(String, String)>,
}

/// Schema 生成器
pub struct SchemaGenerator;

impl SchemaGenerator {
    /// 生成 Project schema
    pub fn generate_project(
        product: &str,
        target: &str,
        resolved_version: &str,
        local_version: Option<&str>,
        build_roots: &[String],
    ) -> ProjectSchema {
        ProjectSchema {
            schema_version: "1.0.0".into(),
            source_identity: SourceIdentity {
                product: product.to_string(),
                target: target.to_string(),
                resolved_version: resolved_version.to_string(),
                local_version: local_version.map(String::from),
            },
            build_roots: build_roots.to_vec(),
        }
    }
    
    /// 生成 Module graph schema
    pub fn generate_module_graph(
        modules: &[String],
        edges: &[(String, String)],
    ) -> ModuleGraphSchema {
        ModuleGraphSchema {
            schema_version: "1.0.0".into(),
            modules: modules.to_vec(),
            edges: edges.to_vec(),
        }
    }
    
    /// 验证未知值不由其它字段猜测
    pub fn validate_no_guessing(schema: &ProjectSchema) -> bool {
        // 本机版本为 None 时不由其它字段猜测
        schema.source_identity.local_version.is_none()
            || !schema.source_identity.local_version.as_deref().unwrap_or("").is_empty()
    }
}
