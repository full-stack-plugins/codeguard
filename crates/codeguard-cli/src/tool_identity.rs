//! 锁定制品的只读解析与内容核对；身份匹配不代表工具可启动或规则已加载。

use crate::tool_lock::LockedTool;
use codeguard_runtime::read_bounded_regular_file;
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Component, Path, PathBuf};

const MAX_TOOL_BYTES: u64 = 128 * 1024 * 1024;
const MAX_BUNDLE_BYTES: u64 = 512 * 1024 * 1024;
const MAX_BUNDLE_ENTRIES: usize = 100_000;

/// 单项制品核对结果，不授予检查能力。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ArtifactVerification {
    /// 规范化后的真实文件路径；缺失时为空。
    pub path: Option<PathBuf>,
    /// 内容摘要是否等于受信工具锁。
    pub digest_matched: bool,
    /// 是否具备基础可执行权限；仍需真实启动验证。
    pub executable_bit: bool,
    /// 确定性失败代码；匹配时为空。
    pub issue: Option<&'static str>,
}

/// 工具及可选运行时的静态制品核对。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LockedArtifactVerification {
    /// 工具启动入口制品。
    pub tool: ArtifactVerification,
    /// 锁要求的运行时；未声明时为空。
    pub runtime: Option<ArtifactVerification>,
    /// 委托发行包目录的静态闭包核对；锁未声明时为空。
    pub bundle: Option<BundleVerification>,
}

/// 目录树制品闭包核对结果。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BundleVerification {
    /// 规范化的目录根；不存在时为空。
    pub path: Option<PathBuf>,
    /// 目录树摘要是否等于锁。
    pub digest_matched: bool,
    /// 失败代码；匹配时为空。
    pub issue: Option<&'static str>,
}

/// 按锁中的来源解析制品，并独立核对工具及运行时字节。
#[must_use]
pub fn verify_locked_artifacts(
    locked: &LockedTool,
    project_root: &Path,
    managed_cache_root: &Path,
    runtime_candidate: Option<&Path>,
) -> LockedArtifactVerification {
    if current_platform_id() != Some(locked.platform.as_str()) {
        return LockedArtifactVerification {
            tool: failed("platform_mismatch"),
            runtime: locked.runtime.as_ref().map(|_| failed("platform_mismatch")),
            bundle: locked
                .bundle
                .as_ref()
                .map(|_| bundle_failed("platform_mismatch")),
        };
    }
    let tool_path = match locked.origin_kind.as_str() {
        "system" => {
            let path = PathBuf::from(&locked.origin_ref);
            if path.is_absolute() {
                Ok(path)
            } else {
                Err("system_origin_not_absolute")
            }
        }
        "project_wrapper" => restricted_join(project_root, &locked.origin_ref),
        "managed_cache" => restricted_join(managed_cache_root, &locked.origin_ref),
        _ => Err("invalid_origin_kind"),
    };
    let tool = match tool_path {
        Ok(path) => verify_artifact(&path, &locked.binary_sha256),
        Err(issue) => failed(issue),
    };
    let runtime = locked.runtime.as_ref().map(|identity| {
        runtime_candidate.map_or_else(
            || failed("runtime_path_missing"),
            |path| verify_artifact(path, &identity.binary_sha256),
        )
    });
    let bundle = locked.bundle.as_ref().map(|identity| {
        let root = match locked.origin_kind.as_str() {
            "system" => {
                let root = PathBuf::from(&identity.root);
                if root.is_absolute() {
                    Ok(root)
                } else {
                    Err("bundle_root_not_absolute")
                }
            }
            "project_wrapper" => restricted_join(project_root, &identity.root),
            "managed_cache" => restricted_join(managed_cache_root, &identity.root),
            _ => Err("invalid_origin_kind"),
        };
        match root {
            Ok(root) => verify_bundle(&root, &identity.tree_sha256),
            Err(issue) => bundle_failed(issue),
        }
    });
    LockedArtifactVerification {
        tool,
        runtime,
        bundle,
    }
}

fn verify_bundle(root: &Path, expected: &str) -> BundleVerification {
    let resolved = match fs::canonicalize(root) {
        Ok(path) => path,
        Err(_) => return bundle_failed("bundle_missing"),
    };
    match hash_bundle_tree(&resolved) {
        Ok(actual) => {
            let digest_matched = actual == expected;
            BundleVerification {
                path: Some(resolved),
                digest_matched,
                issue: if digest_matched {
                    None
                } else {
                    Some("bundle_digest_mismatch")
                },
            }
        }
        Err(issue) => BundleVerification {
            path: Some(resolved),
            digest_matched: false,
            issue: Some(issue),
        },
    }
}

/// 对有界普通目录树计算平台特定的确定性 SHA-256；拒绝链接和特殊文件。
pub fn hash_bundle_tree(root: &Path) -> Result<String, &'static str> {
    let metadata = fs::symlink_metadata(root).map_err(|_| "bundle_missing")?;
    if !metadata.file_type().is_dir() {
        return Err("bundle_not_directory");
    }
    let mut digest = Sha256::new();
    digest.update(b"codeguard-bundle-tree-v1\0");
    let mut pending = vec![root.to_path_buf()];
    let mut entries = 0usize;
    let mut bytes_seen = 0u64;
    while let Some(directory) = pending.pop() {
        let children = fs::read_dir(&directory).map_err(|_| "bundle_unreadable")?;
        let mut children = children
            .map(|entry| {
                entry
                    .map(|entry| entry.path())
                    .map_err(|_| "bundle_unreadable")
            })
            .collect::<Result<Vec<_>, _>>()?;
        children.sort();
        for child in children {
            entries += 1;
            if entries > MAX_BUNDLE_ENTRIES {
                return Err("bundle_entry_limit");
            }
            let relative = child
                .strip_prefix(root)
                .map_err(|_| "bundle_path_invalid")?;
            let relative = relative
                .to_str()
                .ok_or("bundle_non_utf8_path")?
                .replace(std::path::MAIN_SEPARATOR, "/");
            let path_bytes = relative.as_bytes();
            let kind = fs::symlink_metadata(&child)
                .map_err(|_| "bundle_unreadable")?
                .file_type();
            if kind.is_dir() {
                digest.update(b"D");
                digest.update((path_bytes.len() as u64).to_be_bytes());
                digest.update(path_bytes);
                pending.push(child);
            } else if kind.is_file() {
                let content = read_bounded_regular_file(&child, MAX_TOOL_BYTES)
                    .map_err(|_| "bundle_unreadable_or_oversized")?;
                bytes_seen = bytes_seen.saturating_add(content.len() as u64);
                if bytes_seen > MAX_BUNDLE_BYTES {
                    return Err("bundle_byte_limit");
                }
                digest.update(b"F");
                digest.update((path_bytes.len() as u64).to_be_bytes());
                digest.update(path_bytes);
                digest.update((content.len() as u64).to_be_bytes());
                digest.update(Sha256::digest(content));
            } else {
                return Err("bundle_special_or_symlink");
            }
        }
    }
    Ok(format!("{:x}", digest.finalize()))
}

fn bundle_failed(issue: &'static str) -> BundleVerification {
    BundleVerification {
        path: None,
        digest_matched: false,
        issue: Some(issue),
    }
}

/// 返回此构建目标在发行能力矩阵中的规范平台 ID。
#[must_use]
pub fn current_platform_id() -> Option<&'static str> {
    if cfg!(all(target_os = "macos", target_arch = "aarch64")) {
        Some("macos_arm64")
    } else if cfg!(all(target_os = "macos", target_arch = "x86_64")) {
        Some("macos_x86_64")
    } else if cfg!(all(target_os = "linux", target_arch = "x86_64")) {
        Some("linux_x86_64")
    } else if cfg!(all(target_os = "linux", target_arch = "aarch64")) {
        Some("linux_aarch64")
    } else if cfg!(all(target_os = "windows", target_arch = "x86_64")) {
        Some("windows_x86_64")
    } else {
        None
    }
}

fn restricted_join(root: &Path, relative: &str) -> Result<PathBuf, &'static str> {
    let path = Path::new(relative);
    if path.is_absolute()
        || path.as_os_str().is_empty()
        || path
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
    {
        return Err("origin_outside_root");
    }
    let canonical_root = fs::canonicalize(root).map_err(|_| "origin_root_unavailable")?;
    let candidate = root.join(path);
    let canonical_candidate = fs::canonicalize(candidate).map_err(|_| "tool_missing")?;
    if !canonical_candidate.starts_with(canonical_root) {
        return Err("origin_outside_root");
    }
    Ok(canonical_candidate)
}

fn verify_artifact(path: &Path, expected: &str) -> ArtifactVerification {
    let resolved = match fs::canonicalize(path) {
        Ok(path) => path,
        Err(_) => return failed("artifact_missing"),
    };
    let bytes = match read_bounded_regular_file(&resolved, MAX_TOOL_BYTES) {
        Ok(bytes) => bytes,
        Err(_) => {
            return ArtifactVerification {
                path: Some(resolved),
                digest_matched: false,
                executable_bit: false,
                issue: Some("artifact_unreadable_or_oversized"),
            };
        }
    };
    let digest_matched = format!("{:x}", Sha256::digest(&bytes)) == expected;
    let executable_bit = executable(&resolved);
    ArtifactVerification {
        path: Some(resolved),
        digest_matched,
        executable_bit,
        issue: if !digest_matched {
            Some("artifact_digest_mismatch")
        } else if !executable_bit {
            Some("artifact_not_executable")
        } else {
            None
        },
    }
}

#[cfg(unix)]
fn executable(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    fs::metadata(path).is_ok_and(|metadata| metadata.permissions().mode() & 0o111 != 0)
}

#[cfg(not(unix))]
fn executable(path: &Path) -> bool {
    fs::metadata(path).is_ok_and(|metadata| metadata.is_file())
}

fn failed(issue: &'static str) -> ArtifactVerification {
    ArtifactVerification {
        path: None,
        digest_matched: false,
        executable_bit: false,
        issue: Some(issue),
    }
}
