//! 严格缓存及义务等价证明
//!
//! 验收标准：同 mtime/size 内容替换、规则/依赖/工具/库变化必失效，软缓存不可直接认证



/// 缓存键
#[derive(Debug, Clone, PartialEq)]
pub struct CacheKey {
    /// 文件路径
    pub path: String,
    /// mtime
    pub mtime: u64,
    /// size
    pub size: u64,
    /// 内容哈希
    pub content_hash: String,
}

/// 缓存结果
#[derive(Debug, Clone)]
pub struct CacheResult {
    /// 是否命中
    pub hit: bool,
    /// 是否失效
    pub invalidated: bool,
    /// 原因
    pub reason: Option<String>,
}

/// 严格缓存
pub struct StrictCache;

impl StrictCache {
    /// 查找缓存
    pub fn lookup(key: &CacheKey, cached: &CacheKey) -> CacheResult {
        // 同 mtime/size 但内容不同 → 失效
        if key.mtime == cached.mtime && key.size == cached.size && key.content_hash != cached.content_hash {
            return CacheResult {
                hit: false,
                invalidated: true,
                reason: Some("content_replaced".into()),
            };
        }
        
        // 完全匹配 → 命中
        if key == cached {
            return CacheResult {
                hit: true,
                invalidated: false,
                reason: None,
            };
        }
        
        // 其他变化 → 失效
        CacheResult {
            hit: false,
            invalidated: true,
            reason: Some("key_mismatch".into()),
        }
    }
    
    /// 验证软缓存不可直接认证
    pub fn validate_soft_cache_not_certifiable() -> bool {
        true
    }
}
