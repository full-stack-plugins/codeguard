use std::{
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
};

/// 选择本轮 Cargo 入口；保留 rustup 代理名，避免执行解析目标时改变子命令。
/// 返回显式请求或绝对 PATH 中首个普通可执行入口；不执行、不安装，也不故障换工具。
pub(crate) fn resolve_cargo_tool(requested: Option<&Path>) -> Option<PathBuf> {
    if let Some(path) = requested {
        return Some(path.to_path_buf());
    }
    let search_path = std::env::var_os("PATH")?;
    for directory in std::env::split_paths(&search_path) {
        if !directory.is_absolute() {
            continue;
        }
        let Ok(directory) = directory.canonicalize() else {
            continue;
        };
        // 只固定父目录；最终入口可能是名为 cargo 的 rustup 多调用代理。
        let entry = directory.join("cargo");
        if std::fs::metadata(&entry)
            .is_ok_and(|metadata| metadata.is_file() && metadata.permissions().mode() & 0o111 != 0)
        {
            return Some(entry);
        }
    }
    None
}
