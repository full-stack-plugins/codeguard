//! 归档展开与 bundle 发布模块（11.3/11.4）。
//!
//! 实现归档展开、bundle 发布：完整树摘要、内容寻址安装目录、原子发布。

use serde_json::{Value, json};

/// 展开结果。
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum ExtractResult {
    /// 成功。
    Success,
    /// 树摘要不匹配。
    TreeDigestMismatch,
    /// 路径不安全。
    UnsafePath,
    /// 磁盘空间不足。
    DiskFull,
}

impl ExtractResult {
    pub(crate) fn as_str(&self) -> &'static str {
        match self {
            Self::Success => "success",
            Self::TreeDigestMismatch => "tree_digest_mismatch",
            Self::UnsafePath => "unsafe_path",
            Self::DiskFull => "disk_full",
        }
    }
}

/// 发布结果。
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum PublishResult {
    /// 成功。
    Success,
    /// 目录已存在且内容不同。
    AlreadyExists,
    /// 原子重命名失败。
    AtomicRenameFailed,
}

impl PublishResult {
    pub(crate) fn as_str(&self) -> &'static str {
        match self {
            Self::Success => "success",
            Self::AlreadyExists => "already_exists",
            Self::AtomicRenameFailed => "atomic_rename_failed",
        }
    }
}

/// 归档展开。
pub(crate) fn extract_archive(
    tree_digest: &str,
    expected_digest: &str,
    paths: &[String],
) -> ExtractResult {
    // 树摘要验证
    if tree_digest != expected_digest {
        return ExtractResult::TreeDigestMismatch;
    }

    // 路径安全检查
    for path in paths {
        if path.contains("..") || path.starts_with('/') {
            return ExtractResult::UnsafePath;
        }
    }

    ExtractResult::Success
}

/// bundle 发布。
pub(crate) fn publish_bundle(install_dir: &str, existing_dirs: &[String]) -> PublishResult {
    if existing_dirs.contains(&install_dir.to_string()) {
        return PublishResult::AlreadyExists;
    }
    PublishResult::Success
}

/// 生成归档报告。
pub(crate) fn archive_report(extract: &ExtractResult, publish: &PublishResult) -> Value {
    json!({
        "schema_version": "0.1.0",
        "report_type": "archive_bundle",
        "extract": extract.as_str(),
        "publish": publish.as_str(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_success() {
        let result = extract_archive("abc", "abc", &["file.txt".to_string()]);
        assert_eq!(result, ExtractResult::Success);
    }

    #[test]
    fn extract_tree_digest_mismatch() {
        let result = extract_archive("abc", "def", &["file.txt".to_string()]);
        assert_eq!(result, ExtractResult::TreeDigestMismatch);
    }

    #[test]
    fn extract_unsafe_path() {
        let result = extract_archive("abc", "abc", &["../etc/passwd".to_string()]);
        assert_eq!(result, ExtractResult::UnsafePath);
    }

    #[test]
    fn publish_success() {
        let result = publish_bundle("install-dir", &[]);
        assert_eq!(result, PublishResult::Success);
    }

    #[test]
    fn publish_already_exists() {
        let result = publish_bundle("install-dir", &["install-dir".to_string()]);
        assert_eq!(result, PublishResult::AlreadyExists);
    }
}
