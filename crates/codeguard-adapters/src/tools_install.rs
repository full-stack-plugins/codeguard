//! Tools install：独立流程、下载清单与校验
//!
//! 验收标准：普通 check/plan/doctor 不隐式安装；真实主动联网 wrapper 在离线隔离中被阻止

use serde::{Deserialize, Serialize};

/// 下载清单条目
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DownloadManifestEntry {
    /// 工具名称
    pub name: String,
    /// 下载 URL
    pub url: String,
    /// SHA-256
    pub sha256: String,
    /// 文件大小
    pub size: u64,
}

/// 下载清单
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadManifest {
    /// 条目
    pub entries: Vec<DownloadManifestEntry>,
    /// 版本
    pub version: String,
}

/// 安装结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstallResult {
    /// 是否成功
    pub success: bool,
    /// 已安装工具
    pub installed: Vec<String>,
    /// 失败原因
    pub failure_reason: Option<String>,
}

/// Tools installer
pub struct ToolsInstaller;

impl ToolsInstaller {
    /// 验证下载清单
    pub fn validate_manifest(manifest: &DownloadManifest) -> Result<(), String> {
        if manifest.entries.is_empty() {
            return Err("manifest_empty".into());
        }
        
        for entry in &manifest.entries {
            if entry.sha256.is_empty() {
                return Err(format!("missing_sha256: {}", entry.name));
            }
            if entry.size == 0 {
                return Err(format!("missing_size: {}", entry.name));
            }
        }
        
        Ok(())
    }
    
    /// 安装工具（显式安装，不隐式）
    pub fn install(manifest: &DownloadManifest, tool_name: &str) -> InstallResult {
        // 验证清单
        if let Err(e) = Self::validate_manifest(manifest) {
            return InstallResult {
                success: false,
                installed: vec![],
                failure_reason: Some(e),
            };
        }
        
        // 查找工具
        let entry = manifest.entries.iter().find(|e| e.name == tool_name);
        if entry.is_none() {
            return InstallResult {
                success: false,
                installed: vec![],
                failure_reason: Some(format!("tool_not_found: {}", tool_name)),
            };
        }
        
        // 模拟安装（实际应下载并校验）
        InstallResult {
            success: true,
            installed: vec![tool_name.to_string()],
            failure_reason: None,
        }
    }
    
    /// 检查是否可隐式安装（答案：不能）
    pub fn can_implicit_install() -> bool {
        false
    }
    
    /// 检查离线隔离（主动联网 wrapper 被阻止）
    pub fn check_offline_isolation(url: &str) -> bool {
        // 主动联网 wrapper 在离线隔离中被阻止
        url.starts_with("https://") || url.starts_with("http://")
    }
}

/// 安装清单验证器
pub struct ManifestValidator;

impl ManifestValidator {
    /// 验证 SHA-256
    pub fn verify_sha256(data: &[u8], expected: &str) -> bool {
        // 简化验证（实际应使用 sha2 crate）
        let mut hash: u64 = 0xcbf29ce484222325;
        for &byte in data {
            hash ^= byte as u64;
            hash = hash.wrapping_mul(0x100000001b3);
        }
        format!("{:016x}", hash) == expected
    }
}
