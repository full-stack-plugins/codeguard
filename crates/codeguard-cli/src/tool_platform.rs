//! Native executable naming without implicit shell/PATHEXT dispatch.
/// Native runners accept executable images, never batch scripts via an implicit shell.
pub(crate) fn native_tool_name(name: &str) -> String {
    if cfg!(windows) {
        format!("{name}.exe")
    } else {
        name.to_owned()
    }
}
/// Python virtual environments use Scripts on Windows and bin on Unix.
pub(crate) fn venv_bin_directory() -> &'static str {
    if cfg!(windows) { "Scripts" } else { "bin" }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn platform_native_filename_is_explicit() {
        assert_eq!(
            native_tool_name("ruff"),
            if cfg!(windows) { "ruff.exe" } else { "ruff" }
        );
        assert_eq!(
            native_tool_name("cargo"),
            if cfg!(windows) { "cargo.exe" } else { "cargo" }
        );
    }
    #[test]
    fn virtualenv_uses_platform_layout() {
        assert_eq!(
            venv_bin_directory(),
            if cfg!(windows) { "Scripts" } else { "bin" }
        );
    }
}

#[cfg(test)]
mod cwd_tests {
    #[test]
    fn native_cwd_is_the_absolute_volume_root_not_the_project() {
        let executable = std::env::current_exe().unwrap();
        let root = super::isolated_native_cwd(&executable).unwrap();
        assert!(root.is_absolute());
        assert!(root.parent().is_none());
        assert!(executable.starts_with(&root));
        assert_ne!(Some(root.as_path()), executable.parent());
        assert!(super::isolated_native_cwd(std::path::Path::new("relative/tool")).is_none());
    }
}

/// Use the executable's absolute volume root, never the inspected project cwd.
/// On Windows a bare slash is drive-relative and is rejected by ProcessSpec.
pub(crate) fn isolated_native_cwd(executable: &std::path::Path) -> Option<std::path::PathBuf> {
    if !executable.is_absolute() {
        return None;
    }
    executable
        .ancestors()
        .last()
        .filter(|root| root.is_absolute())
        .map(std::path::Path::to_path_buf)
}
