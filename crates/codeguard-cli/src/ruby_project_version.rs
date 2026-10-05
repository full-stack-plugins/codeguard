//! Ruby 固定解析器的有界项目版本约束；声明不适用时不制造源码诊断。
use codeguard_runtime::read_bounded_regular_file;
use serde_json::Value;
use std::{
    io::ErrorKind,
    path::{Component, Path, PathBuf},
    sync::atomic::{AtomicBool, Ordering},
    time::Instant,
};

/// 获取单文件入口的受限项目根；按工作台、Git、Gemfile优先级寻找最近标记，不执行配置。
pub(crate) fn source_root(source: &Path) -> Option<PathBuf> {
    let absolute = if source.is_absolute() {
        source.to_path_buf()
    } else {
        std::env::current_dir().ok()?.join(source)
    };
    let parent = absolute.parent()?;
    Some(
        [".codeguard", ".git", "Gemfile"]
            .iter()
            .find_map(|name| {
                parent
                    .ancestors()
                    .take(64)
                    .find(|p| std::fs::symlink_metadata(p.join(name)).is_ok())
            })
            .unwrap_or(parent)
            .to_path_buf(),
    )
}

/// 版本观察输入，保留已有声明及更近目录中尚不存在的声明路径。
type VersionInputs = Vec<(PathBuf, Option<Vec<u8>>)>;

/// 读取源码祖先内最近版本声明及所有更近缺项；返回原始字节快照或环境原因。
fn capture(root: &Path, source: &Path) -> Result<VersionInputs, &'static str> {
    if !root.is_absolute()
        || source
            .components()
            .any(|c| matches!(c, Component::ParentDir))
        || !source.starts_with(root)
    {
        return Err("ruby_project_version_unresolved");
    }
    let mut inputs = Vec::new();
    for parent in source
        .parent()
        .ok_or("ruby_project_version_unresolved")?
        .ancestors()
        .take(64)
    {
        if !parent.starts_with(root) {
            break;
        }
        let mut prefix = PathBuf::new();
        for component in parent.components() {
            prefix.push(component);
            if std::fs::symlink_metadata(&prefix)
                .map_err(|_| "ruby_project_version_unreadable")?
                .file_type()
                .is_symlink()
            {
                return Err("ruby_project_version_unreadable");
            }
        }
        let path = parent.join(".ruby-version");
        match std::fs::symlink_metadata(&path) {
            Err(e) if e.kind() == ErrorKind::NotFound => inputs.push((path, None)),
            Err(_) => return Err("ruby_project_version_unreadable"),
            Ok(_) => {
                let bytes = read_bounded_regular_file(&path, 4096)
                    .map_err(|_| "ruby_project_version_unreadable")?;
                let text = std::str::from_utf8(&bytes)
                    .map_err(|_| "ruby_project_version_unresolved")?
                    .trim();
                let version = text.strip_prefix("ruby-").unwrap_or(text);
                if !matches!(version, "2.6.10" | "2.6.10p210") {
                    let numeric = version.split('.').collect::<Vec<_>>();
                    return Err(
                        if numeric.len() == 3
                            && numeric
                                .iter()
                                .all(|p| !p.is_empty() && p.bytes().all(|b| b.is_ascii_digit()))
                        {
                            "ruby_project_version_mismatch"
                        } else {
                            "ruby_project_version_unresolved"
                        },
                    );
                }
                inputs.push((path, Some(bytes)));
                return Ok(inputs);
            }
        }
        if parent == root {
            return Ok(inputs);
        }
    }
    Err("ruby_project_version_unresolved")
}

/// 仅核对当前明确声明对固定解析器是否适用；无声明仍是未批准的初步观察。
pub(crate) fn compatible(root: &Path, source: &Path) -> bool {
    capture(root, source).is_ok()
}

/// 调用固定Ruby解析器并复核版本输入；参数绑定项目根、源码路径、字节和共同预算。
pub(crate) fn observe(
    root: &Path,
    source: &Path,
    tool: &Path,
    bytes: &[u8],
    deadline: Instant,
    cancelled: &AtomicBool,
) -> Value {
    if cancelled.load(Ordering::Relaxed) || codeguard_runtime::sigint_cancellation_requested() {
        return crate::ruby_lint_command::unavailable("request_cancelled");
    }
    if Instant::now() >= deadline {
        return crate::ruby_lint_command::unavailable("request_deadline_exceeded");
    }
    let original = match capture(root, source) {
        Ok(inputs) => inputs,
        Err(reason) => return crate::ruby_lint_command::unavailable(reason),
    };
    let report = crate::ruby_syntax_probe::observe(tool, bytes, deadline, cancelled);
    if capture(root, source).ok().as_ref() != Some(&original) {
        crate::ruby_lint_command::unavailable("ruby_project_version_changed_during_check")
    } else {
        report
    }
}
