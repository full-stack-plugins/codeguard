use std::{
    fs,
    io::ErrorKind,
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
    for component in [".venv", crate::tool_platform::venv_bin_directory()] {
        directory.push(component);
        match fs::symlink_metadata(&directory) {
            Ok(metadata)
                if metadata.is_dir() && crate::platform_fs::file_identity(&directory).is_ok() => {}
            Err(error) if error.kind() == ErrorKind::NotFound => {
                local_directories_present = false;
                break;
            }
            // 目录链接或坏环境不能被全局 Ruff 掩盖；无配置时不会调用本函数。
            _ => return Err("ruff_local_tool_invalid"),
        }
    }
    if local_directories_present {
        let entry = directory.join(crate::tool_platform::native_tool_name("ruff"));
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
        .map(|directory| directory.join(crate::tool_platform::native_tool_name("ruff")))
        .find_map(|candidate| {
            candidate
                .canonicalize()
                .ok()
                .filter(|path| is_executable_file(path))
        }))
}

fn is_executable_file(path: &Path) -> bool {
    crate::platform_fs::is_executable(path)
}

#[cfg(test)]
mod portable_tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};
    static NEXT: AtomicU64 = AtomicU64::new(0);
    #[test]
    fn platform_virtualenv_selects_exact_native_entry_and_rejects_directory() {
        let root = std::env::temp_dir().canonicalize().unwrap().join(format!(
            "cg-ruff-native-layout-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        crate::platform_fs::create_private_dir(&root).unwrap();
        let venv = root.join(".venv");
        crate::platform_fs::create_private_dir(&venv).unwrap();
        let bin = venv.join(crate::tool_platform::venv_bin_directory());
        crate::platform_fs::create_private_dir(&bin).unwrap();
        let entry = bin.join(crate::tool_platform::native_tool_name("ruff"));
        std::fs::copy(std::env::current_exe().unwrap(), &entry).unwrap();
        #[cfg(unix)]
        crate::platform_fs::set_private_permissions(&entry, 0o700).unwrap();
        assert_eq!(
            resolve_ruff_tool(&root, None),
            Ok(Some(entry.canonicalize().unwrap()))
        );
        std::fs::remove_file(&entry).unwrap();
        std::fs::create_dir(&entry).unwrap();
        assert_eq!(
            resolve_ruff_tool(&root, None),
            Err("ruff_local_tool_invalid")
        );
        std::fs::remove_dir_all(root).unwrap();
    }
}
