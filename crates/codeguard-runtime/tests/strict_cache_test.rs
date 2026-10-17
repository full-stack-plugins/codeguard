//! 严格缓存测试

use codeguard_runtime::strict_cache::*;

fn create_key(path: &str, mtime: u64, size: u64, hash: &str) -> CacheKey {
    CacheKey {
        path: path.into(),
        mtime,
        size,
        content_hash: hash.into(),
    }
}

#[test]
fn cache_hit() {
    let key = create_key("file.rs", 100, 1000, "hash-1");
    let cached = create_key("file.rs", 100, 1000, "hash-1");
    
    let result = StrictCache::lookup(&key, &cached);
    assert!(result.hit);
    assert!(!result.invalidated);
}

#[test]
fn cache_invalidated_same_mtime_size() {
    let key = create_key("file.rs", 100, 1000, "hash-2");
    let cached = create_key("file.rs", 100, 1000, "hash-1");
    
    let result = StrictCache::lookup(&key, &cached);
    assert!(!result.hit);
    assert!(result.invalidated);
    assert_eq!(result.reason, Some("content_replaced".into()));
}

#[test]
fn cache_invalidated_key_mismatch() {
    let key = create_key("file.rs", 200, 1000, "hash-1");
    let cached = create_key("file.rs", 100, 1000, "hash-1");
    
    let result = StrictCache::lookup(&key, &cached);
    assert!(!result.hit);
    assert!(result.invalidated);
}

#[test]
fn validate_soft_cache_not_certifiable() {
    assert!(StrictCache::validate_soft_cache_not_certifiable());
}
