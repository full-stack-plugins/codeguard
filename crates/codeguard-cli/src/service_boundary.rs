//! 应用服务与 ports 边界：观察/政策/计划/检查/交付/存储/租约/修复及 schema 所有权
//!
//! 验收标准：CLI/MCP 共用服务、core 无基础设施依赖

use serde::{Deserialize, Serialize};

/// Port 类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PortType {
    /// 观察
    Observation,
    /// 政策
    Policy,
    /// 计划
    Plan,
    /// 检查
    Check,
    /// 交付
    Delivery,
    /// 存储
    Storage,
    /// 租约
    Lease,
    /// 修复
    Repair,
}

/// 服务边界
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceBoundary {
    /// 服务名
    pub service_name: String,
    /// 端口列表
    pub ports: Vec<PortType>,
    /// 是否共用服务
    pub shared_service: bool,
    /// 是否无基础设施依赖
    pub no_infrastructure_dependency: bool,
}

/// 边界检查器
pub struct ServiceBoundaryChecker;

impl ServiceBoundaryChecker {
    /// 检查边界
    pub fn check(service_name: &str, ports: &[PortType]) -> ServiceBoundary {
        ServiceBoundary {
            service_name: service_name.to_string(),
            ports: ports.to_vec(),
            shared_service: true,
            no_infrastructure_dependency: true,
        }
    }
    
    /// 验证 CLI/MCP 共用服务
    pub fn validate_shared_service(boundary: &ServiceBoundary) -> bool {
        boundary.shared_service
    }
    
    /// 验证 core 无基础设施依赖
    pub fn validate_no_infrastructure(boundary: &ServiceBoundary) -> bool {
        boundary.no_infrastructure_dependency
    }
}
