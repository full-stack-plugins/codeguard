//! 单文件检查前的项目本地 ESLint 候选识别；只读且不执行配置。

use codeguard_adapters::{EslintConfigState, inspect_eslint_local_candidate};
use codeguard_core::ObservedPathKind;
use codeguard_runtime::read_bounded_regular_file;
use std::path::Path;

/// 识别源码最近的项目根是否具备可进一步原生探测的本地 ESLint 与配置。
/// 参数为普通源码路径；返回真仅表示应先确认原生调用，不证明工具可执行或规则有效。
pub(crate) fn observed_candidate(source: &Path) -> bool {
    let Ok(source) = source.canonicalize() else {
        return false;
    };
    let Some(parent) = source.parent() else {
        return false;
    };
    for root in parent.ancestors() {
        let manifest = root.join("package.json");
        if !regular_file(&manifest) {
            continue;
        }
        let Ok(project_manifest) = read_bounded_regular_file(&manifest, 256 * 1024) else {
            return false;
        };
        let local_root = root.join("node_modules");
        let package_root = local_root.join("eslint");
        if !regular_directory(&local_root) || !regular_directory(&package_root) {
            return false;
        }
        let local_manifest = package_root.join("package.json");
        let entry = package_root.join("bin/eslint.js");
        if !regular_file(&local_manifest) || !regular_file(&entry) {
            return false;
        }
        let Ok(local_manifest) = read_bounded_regular_file(&local_manifest, 256 * 1024) else {
            return false;
        };
        // 仅把 ESLint 10 可直接选取的普通 flat config 作为候选；旧配置和 TS loader 须另行确认。
        let config_count = ["eslint.config.js", "eslint.config.mjs", "eslint.config.cjs"]
            .iter()
            .filter(|name| {
                let path = root.join(name);
                regular_file(&path) && read_bounded_regular_file(&path, 1024 * 1024).is_ok()
            });
        if config_count.count() != 1 {
            return false;
        }
        let candidate = inspect_eslint_local_candidate(
            Some(&project_manifest),
            Some(&local_manifest),
            Some(ObservedPathKind::File),
            EslintConfigState::Observed,
        );
        return candidate.state == "local_candidate_requires_native_probe";
    }
    false
}

fn regular_file(path: &Path) -> bool {
    std::fs::symlink_metadata(path).is_ok_and(|metadata| metadata.is_file())
}

fn regular_directory(path: &Path) -> bool {
    std::fs::symlink_metadata(path).is_ok_and(|metadata| metadata.is_dir())
}
