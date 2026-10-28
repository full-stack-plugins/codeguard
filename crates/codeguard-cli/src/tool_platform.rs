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
