use std::{
    fs,
    io::ErrorKind,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
};

/// 为已配置根选择 Ruff；参数为受检根和显式入口，返回固定工具或具体本地环境阻塞。
/// 显式入口优先，普通根内 .venv 次之，再查绝对 PATH；不执行、不安装、不故障换工具。
pub(crate) fn resolve_ruff_tool(
    root: &Path,
    requested: Option<&Path>,
) -> Result<Option<PathBuf>, &'static str> {
    if let Some(path) = requested {
        return Ok(path
            .canonicalize()
            .ok()
            .filter(|path| is_executable_file(path)));
    }
    let mut directory = root.to_path_buf();
    let mut local_directories_present = true;
    for component in [".venv", "bin"] {
        directory.push(component);
        match fs::symlink_metadata(&directory) {
            Ok(metadata) if metadata.is_dir() => {}
            Err(error) if error.kind() == ErrorKind::NotFound => {
                local_directories_present = false;
                break;
            }
            // 目录链接或坏环境不能被全局 Ruff 掩盖；无配置时不会调用本函数。
            _ => return Err("ruff_local_tool_invalid"),
        }
    }
    if local_directories_present {
        let entry = directory.join("ruff");
        match fs::symlink_metadata(&entry) {
            Err(error) if error.kind() == ErrorKind::NotFound => {}
            Err(_) => return Err("ruff_local_tool_invalid"),
            Ok(_) => {
                return entry
                    .canonicalize()
                    .ok()
                    .filter(|path| is_executable_file(path))
                    .map(Some)
                    .ok_or("ruff_local_tool_invalid");
            }
        }
    }
    let Some(search_path) = std::env::var_os("PATH") else {
        return Ok(None);
    };
    Ok(std::env::split_paths(&search_path)
        .filter(|directory| directory.is_absolute())
        .map(|directory| directory.join("ruff"))
        .find_map(|candidate| {
            candidate
                .canonicalize()
                .ok()
                .filter(|path| is_executable_file(path))
        }))
}

fn is_executable_file(path: &Path) -> bool {
    fs::metadata(path)
        .is_ok_and(|metadata| metadata.is_file() && metadata.permissions().mode() & 0o111 != 0)
}
