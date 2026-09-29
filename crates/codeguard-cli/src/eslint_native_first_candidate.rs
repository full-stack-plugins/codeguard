//! 单文件检查前的项目本地 ESLint 候选识别；只读且不执行配置。

use codeguard_adapters::{EslintConfigState, inspect_eslint_local_candidate};
use codeguard_core::ObservedPathKind;
use codeguard_runtime::read_bounded_regular_file;
use std::{
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
};

/// 已观察到的项目本地 ESLint 10 上下文，尚不代表原生执行成功。
pub(crate) struct LocalEslintCandidate {
    /// 项目构建根，同时是原生检查的工作目录。
    pub(crate) root: PathBuf,
    /// 本地 ESLint JS 入口。
    pub(crate) entry: PathBuf,
    /// 唯一的普通 flat config。
    pub(crate) config: PathBuf,
    /// 本地包清单中观察到的具体版本，须由原生版本探针复核。
    pub(crate) version: String,
}

/// 识别源码最近项目根的本地 ESLint 包、入口和单一 flat config。
/// 参数为普通源码路径；返回候选仅表示可以尝试原生版本探测，不证明规则有效。
pub(crate) fn observed_candidate(source: &Path) -> Option<LocalEslintCandidate> {
    let source = source.canonicalize().ok()?;
    let parent = source.parent()?;
    for root in parent.ancestors() {
        let manifest = root.join("package.json");
        if !regular_file(&manifest) {
            continue;
        }
        let project_manifest = read_bounded_regular_file(&manifest, 256 * 1024).ok()?;
        let local_root = root.join("node_modules");
        let package_root = local_root.join("eslint");
        if !regular_directory(&local_root) || !regular_directory(&package_root) {
            return None;
        }
        let local_manifest = package_root.join("package.json");
        let entry = package_root.join("bin/eslint.js");
        if !regular_file(&local_manifest) || !regular_file(&entry) {
            return None;
        }
        let local_manifest = read_bounded_regular_file(&local_manifest, 256 * 1024).ok()?;
        // 旧配置、TS loader 与多配置选择尚未受控，不能构造自动原生命令。
        let configs: Vec<_> = ["eslint.config.js", "eslint.config.mjs", "eslint.config.cjs"]
            .iter()
            .map(|name| root.join(name))
            .filter(|path| std::fs::symlink_metadata(path).is_ok())
            .collect();
        let [config] = configs.as_slice() else {
            return None;
        };
        if !regular_file(config) || read_bounded_regular_file(config, 1024 * 1024).is_err() {
            return None;
        }
        let candidate = inspect_eslint_local_candidate(
            Some(&project_manifest),
            Some(&local_manifest),
            Some(ObservedPathKind::File),
            EslintConfigState::Observed,
        );
        if candidate.state != "local_candidate_requires_native_probe" {
            return None;
        }
        return Some(LocalEslintCandidate {
            root: root.to_path_buf(),
            entry,
            config: config.clone(),
            version: candidate.observed_version?,
        });
    }
    None
}

/// 从当前进程 PATH 选择普通可执行 Node 文件；缺失只表示当前调用未解析到运行时。
/// 返回路径会被原生探针按字节冻结，不能据此宣称项目工具不存在或检查完成。
pub(crate) fn node_on_path() -> Option<PathBuf> {
    for directory in std::env::split_paths(&std::env::var_os("PATH")?) {
        if !directory.is_absolute() {
            continue;
        }
        let node = directory.join("node");
        let Ok(node) = node.canonicalize() else {
            continue;
        };
        let Ok(metadata) = std::fs::symlink_metadata(&node) else {
            continue;
        };
        if metadata.is_file()
            && metadata.permissions().mode() & 0o111 != 0
            && read_bounded_regular_file(&node, 128 * 1024 * 1024).is_ok()
        {
            return Some(node);
        }
    }
    None
}

fn regular_file(path: &Path) -> bool {
    std::fs::symlink_metadata(path).is_ok_and(|metadata| metadata.is_file())
}

fn regular_directory(path: &Path) -> bool {
    std::fs::symlink_metadata(path).is_ok_and(|metadata| metadata.is_dir())
}
