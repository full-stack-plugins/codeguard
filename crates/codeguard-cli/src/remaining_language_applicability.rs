//! 剩余语言六类别适用性（ObjC/Nix/CUDA/Liquid 等）
//!
//! 验收标准：每个槽位有实际依据，not_applicable 不得用缺工具解释

use serde::{Deserialize, Serialize};

/// 适用性槽位
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SlotStatus {
    /// 适用
    Applicable,
    /// 不适用
    NotApplicable,
    /// 缺口
    Gap,
}

/// 槽位
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApplicabilitySlot {
    /// 类别
    pub category: String,
    /// 状态
    pub status: SlotStatus,
    /// 依据
    pub rationale: String,
}

/// 剩余语言适用性检查器
pub struct RemainingLanguageApplicability;

impl RemainingLanguageApplicability {
    /// 检查 Nix 适用性
    pub fn check_nix() -> Vec<ApplicabilitySlot> {
        vec![
            ApplicabilitySlot { category: "lint".into(), status: SlotStatus::Applicable, rationale: "nix-instantiate --parse 可用".into() },
            ApplicabilitySlot { category: "comments".into(), status: SlotStatus::Applicable, rationale: "Nix Doc 可用".into() },
            ApplicabilitySlot { category: "dependencies".into(), status: SlotStatus::Applicable, rationale: "flake.nix 可用".into() },
            ApplicabilitySlot { category: "cve".into(), status: SlotStatus::Gap, rationale: "无专用 CVE 工具".into() },
            ApplicabilitySlot { category: "security".into(), status: SlotStatus::Gap, rationale: "无专用安全工具".into() },
            ApplicabilitySlot { category: "build".into(), status: SlotStatus::Applicable, rationale: "nix-build 可用".into() },
        ]
    }
    
    /// 检查 CUDA 适用性
    pub fn check_cuda() -> Vec<ApplicabilitySlot> {
        vec![
            ApplicabilitySlot { category: "lint".into(), status: SlotStatus::Applicable, rationale: "nvcc --Werror 可用".into() },
            ApplicabilitySlot { category: "comments".into(), status: SlotStatus::Applicable, rationale: "Doxygen 可用".into() },
            ApplicabilitySlot { category: "dependencies".into(), status: SlotStatus::Applicable, rationale: "CMake 可用".into() },
            ApplicabilitySlot { category: "cve".into(), status: SlotStatus::Gap, rationale: "无专用 CVE 工具".into() },
            ApplicabilitySlot { category: "security".into(), status: SlotStatus::Gap, rationale: "无专用安全工具".into() },
            ApplicabilitySlot { category: "build".into(), status: SlotStatus::Applicable, rationale: "nvcc 可用".into() },
        ]
    }
    
    /// 检查 Liquid 适用性
    pub fn check_liquid() -> Vec<ApplicabilitySlot> {
        vec![
            ApplicabilitySlot { category: "lint".into(), status: SlotStatus::Applicable, rationale: "liquidjs 可用".into() },
            ApplicabilitySlot { category: "comments".into(), status: SlotStatus::Applicable, rationale: "Liquid 注释可用".into() },
            ApplicabilitySlot { category: "dependencies".into(), status: SlotStatus::Gap, rationale: "无专用依赖工具".into() },
            ApplicabilitySlot { category: "cve".into(), status: SlotStatus::Gap, rationale: "无专用 CVE 工具".into() },
            ApplicabilitySlot { category: "security".into(), status: SlotStatus::Gap, rationale: "无专用安全工具".into() },
            ApplicabilitySlot { category: "build".into(), status: SlotStatus::Applicable, rationale: "Jekyll/Hugo 可用".into() },
        ]
    }
    
    /// 验证 not_applicable 不用缺工具解释
    pub fn validate_no_missing_tool_excuse(slots: &[ApplicabilitySlot]) -> bool {
        slots.iter().all(|s| s.status != SlotStatus::NotApplicable || !s.rationale.contains("缺工具"))
    }
}
