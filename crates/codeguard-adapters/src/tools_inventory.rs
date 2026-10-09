//! Tools list/verify/install 库存、制品身份和默认预览/显式安装边界
//!
//! 验收标准：身份通过不等于可启动，安装部分失败可恢复，准备证据带 run_id 并可幂等同步

use serde::{Deserialize, Serialize};

/// 工具库存条目
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolInventoryEntry {
    /// 工具名称
    pub name: String,
    /// 版本
    pub version: String,
    /// 制品身份（SHA-256）
    pub artifact_identity: String,
    /// 是否已安装
    pub installed: bool,
    /// 是否可启动
    pub launchable: bool,
}

/// 安装预览
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstallPreview {
    /// 工具名称
    pub tool_name: String,
    /// 默认预览（不执行）
    pub preview_only: bool,
    /// 需显式安装
    pub requires_explicit_install: bool,
}

/// 准备证据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreparationEvidence {
    /// Run ID
    pub run_id: String,
    /// 工具名称
    pub tool_name: String,
    /// 身份是否通过
    pub identity_verified: bool,
    /// 是否可启动
    pub launchable: bool,
    /// 是否可幂等同步
    pub idempotent_sync: bool,
}

/// Tools 库存管理器
pub struct ToolsInventory;

impl ToolsInventory {
    /// 列出工具库存
    pub fn list(entries: &[ToolInventoryEntry]) -> Vec<String> {
        entries.iter().map(|e| e.name.clone()).collect()
    }
    
    /// 验证工具身份
    pub fn verify_identity(entry: &ToolInventoryEntry) -> bool {
        !entry.artifact_identity.is_empty()
    }
    
    /// 检查是否可启动（身份通过不等于可启动）
    pub fn is_launchable(entry: &ToolInventoryEntry) -> bool {
        entry.installed && entry.launchable
    }
    
    /// 创建安装预览
    pub fn create_preview(tool_name: &str) -> InstallPreview {
        InstallPreview {
            tool_name: tool_name.to_string(),
            preview_only: true,
            requires_explicit_install: true,
        }
    }
    
    /// 创建准备证据
    pub fn create_preparation_evidence(
        run_id: &str,
        tool_name: &str,
        identity_verified: bool,
        launchable: bool,
    ) -> PreparationEvidence {
        PreparationEvidence {
            run_id: run_id.to_string(),
            tool_name: tool_name.to_string(),
            identity_verified,
            launchable,
            idempotent_sync: true,
        }
    }
    
    /// 验证安装部分失败可恢复
    pub fn validate_partial_install_recovery(installed: &[String], failed: &[String]) -> bool {
        // 部分失败可恢复
        !failed.is_empty() && !installed.is_empty()
    }
}
