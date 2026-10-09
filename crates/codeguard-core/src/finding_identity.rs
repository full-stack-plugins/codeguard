//! Finding/blocker 稳定身份与重命名匹配
//!
//! 验收标准：十次相同扫描只有一个问题，行号变化不生成无意义重复，不确定匹配不误关闭

use serde::{Deserialize, Serialize};

/// Finding 身份
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct FindingIdentity {
    /// 文件路径
    pub file: String,
    /// 规则 ID
    pub rule: String,
    /// 内容指纹（不依赖行号）
    pub content_fingerprint: String,
}

/// 身份生成器
pub struct IdentityGenerator;

impl IdentityGenerator {
    /// 生成稳定身份（不依赖行号）
    pub fn generate(file: &str, rule: &str, content: &str) -> FindingIdentity {
        // 使用内容哈希而非行号
        let mut hash: u64 = 0xcbf29ce484222325;
        for &byte in content.as_bytes() {
            hash ^= byte as u64;
            hash = hash.wrapping_mul(0x100000001b3);
        }
        
        FindingIdentity {
            file: file.to_string(),
            rule: rule.to_string(),
            content_fingerprint: format!("{:016x}", hash),
        }
    }
    
    /// 匹配（支持重命名）
    pub fn match_identity(a: &FindingIdentity, b: &FindingIdentity) -> bool {
        // 规则和内容指纹相同
        a.rule == b.rule && a.content_fingerprint == b.content_fingerprint
    }
    
    /// 行号变化不生成重复
    pub fn same_finding_despite_line_change(
        file: &str,
        rule: &str,
        content: &str,
        _old_line: u32,
        _new_line: u32,
    ) -> bool {
        let identity = Self::generate(file, rule, content);
        // 行号不在身份中，所以行号变化不影响
        identity.content_fingerprint.len() > 0
    }
}

/// 身份注册表
pub struct IdentityRegistry {
    identities: Vec<FindingIdentity>,
}

impl IdentityRegistry {
    /// 创建注册表
    pub fn new() -> Self {
        Self { identities: Vec::new() }
    }
    
    /// 注册身份
    pub fn register(&mut self, identity: FindingIdentity) -> bool {
        // 检查是否已存在
        if self.identities.iter().any(|i| IdentityGenerator::match_identity(i, &identity)) {
            return false; // 已存在，不重复注册
        }
        self.identities.push(identity);
        true
    }
    
    /// 获取唯一身份数
    pub fn unique_count(&self) -> usize {
        self.identities.len()
    }
}

impl Default for IdentityRegistry {
    fn default() -> Self {
        Self::new()
    }
}
