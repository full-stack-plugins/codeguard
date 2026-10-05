//! 固定 Go 语法 SDK 的静态项目版本边界；不解析依赖或自动下载工具链。
use codeguard_runtime::read_bounded_regular_file;
use serde_json::Value;
use std::{
    io::ErrorKind,
    path::{Component, Path, PathBuf},
    sync::atomic::{AtomicBool, Ordering},
    time::Instant,
};

type VersionInputs = Vec<(PathBuf, Option<Vec<u8>>)>;

// go 是最低版本，toolchain 是建议入口；此候选只支持固定1.23.4，不冒充完整选择器。
fn check_declarations(bytes: &[u8]) -> Result<(), &'static str> {
    let text = std::str::from_utf8(bytes).map_err(|_| "go_project_version_unresolved")?;
    if text.contains("/*") || text.contains("*/") {
        return Err("go_project_version_unresolved");
    }
    let mut go_seen = false;
    let mut toolchain_seen = false;
    for line in text.lines() {
        let fields = line
            .split("//")
            .next()
            .unwrap_or("")
            .split_whitespace()
            .collect::<Vec<_>>();
        let Some(key) = fields.first() else {
            continue;
        };
        if !matches!(*key, "go" | "toolchain") {
            continue;
        }
        let seen = if *key == "go" {
            &mut go_seen
        } else {
            &mut toolchain_seen
        };
        if *seen || fields.len() != 2 {
            return Err("go_project_version_unresolved");
        }
        *seen = true;
        if *key == "toolchain" && fields[1] == "default" {
            continue;
        }
        let version = if *key == "toolchain" {
            fields[1]
                .strip_prefix("go")
                .ok_or("go_project_version_unresolved")?
        } else {
            fields[1]
        };
        let parts = version.split('.').collect::<Vec<_>>();
        if !(2..=3).contains(&parts.len())
            || parts.iter().any(|p| {
                p.is_empty()
                    || !p.bytes().all(|b| b.is_ascii_digit())
                    || (p.len() > 1 && p.starts_with('0'))
            })
        {
            return Err("go_project_version_unresolved");
        }
        let numbers = parts
            .iter()
            .map(|p| p.parse::<u32>())
            .collect::<Result<Vec<_>, _>>()
            .map_err(|_| "go_project_version_unresolved")?;
        if (numbers[0], numbers[1], *numbers.get(2).unwrap_or(&0)) > (1, 23, 4) {
            return Err(if *key == "go" {
                "go_project_version_mismatch"
            } else {
                "go_project_toolchain_mismatch"
            });
        }
    }
    Ok(())
}

fn capture(root: &Path, source: &Path) -> Result<VersionInputs, &'static str> {
    if !root.is_absolute()
        || !source.starts_with(root)
        || source
            .components()
            .any(|c| matches!(c, Component::ParentDir))
    {
        return Err("go_project_version_unresolved");
    }
    let mut inputs = Vec::new();
    let mut module_found = false;
    let mut workspace_found = false;
    for parent in source
        .parent()
        .ok_or("go_project_version_unresolved")?
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
                .map_err(|_| "go_project_version_unreadable")?
                .file_type()
                .is_symlink()
            {
                return Err("go_project_version_unreadable");
            }
        }
        for name in ["go.mod", "go.work"] {
            if (name == "go.mod" && module_found) || (name == "go.work" && workspace_found) {
                continue;
            }
            let path = parent.join(name);
            match std::fs::symlink_metadata(&path) {
                Err(e) if e.kind() == ErrorKind::NotFound => inputs.push((path, None)),
                Err(_) => return Err("go_project_version_unreadable"),
                Ok(_) => {
                    let bytes = read_bounded_regular_file(&path, 256 * 1024)
                        .map_err(|_| "go_project_version_unreadable")?;
                    check_declarations(&bytes)?;
                    inputs.push((path, Some(bytes)));
                    if name == "go.mod" {
                        module_found = true;
                    } else {
                        workspace_found = true;
                    }
                }
            }
        }
        if parent == root {
            return Ok(inputs);
        }
    }
    Err("go_project_version_unresolved")
}

/// 只核对当前项目声明是否允许固定 SDK 的初步语法观察，未证明语言版本语义。
pub(crate) fn compatible(root: &Path, source: &Path) -> bool {
    capture(root, source).is_ok()
}

/// 绑定项目根、源路径和冻结源码调用 SDK；取消、预算及声明变化保留环境未完成。
pub(crate) fn observe(
    root: &Path,
    source: &Path,
    tool: &Path,
    bytes: &[u8],
    deadline: Instant,
    cancelled: &AtomicBool,
) -> Value {
    if cancelled.load(Ordering::Relaxed) || codeguard_runtime::sigint_cancellation_requested() {
        return unavailable("request_cancelled");
    }
    if Instant::now() >= deadline {
        return unavailable("request_deadline_exceeded");
    }
    let original = match capture(root, source) {
        Ok(v) => v,
        Err(reason) => return unavailable(reason),
    };
    let report = crate::go_syntax_probe::observe(tool, bytes, deadline, cancelled);
    if capture(root, source).ok().as_ref() != Some(&original) {
        unavailable("go_project_version_changed_during_check")
    } else {
        report
    }
}
fn unavailable(reason: &str) -> Value {
    let mut report = crate::go_syntax_probe::unavailable(reason);
    report["status"] = serde_json::json!("incomplete");
    report
}

#[cfg(test)]
mod tests {
    use super::check_declarations;
    #[test]
    fn go_minimum_and_suggested_toolchain_are_distinct() {
        for good in [
            "go 1.23\n",
            "go 1.22\ntoolchain go1.23.4\n",
            "go 1.23.4 // note\ntoolchain default\n",
            "module sample\nrequire (\n foo/bar v1.0.0\n)\n",
        ] {
            assert_eq!(check_declarations(good.as_bytes()), Ok(()), "{good}");
        }
        assert_eq!(
            check_declarations(b"go 1.24.0\n"),
            Err("go_project_version_mismatch")
        );
        assert_eq!(
            check_declarations(b"go 1.23\ntoolchain go1.23.5\n"),
            Err("go_project_toolchain_mismatch")
        );
        for bad in [
            "go 1.23\ngo 1.22",
            "toolchain go1.23rc1",
            "go (1.23)",
            "go 01.23",
            "/* go 1.24 */",
            "go 1.23 extra",
        ] {
            assert_eq!(
                check_declarations(bad.as_bytes()),
                Err("go_project_version_unresolved"),
                "{bad}"
            );
        }
    }
}
