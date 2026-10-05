use serde_json::{Value, json};
use std::{
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
};

/// 本轮 Go 原生工具选择；显式无效工具保留，不静默改用 PATH。
pub(crate) enum GoToolSelection {
    Explicit(PathBuf),
    Path(PathBuf),
    NotFound,
}

impl GoToolSelection {
    /// 从显式路径或调用进程 PATH 选择一次工具；不启动进程、不安装工具。
    /// 显式路径存在时固定规范路径；自动选择仅使用绝对目录中的普通可执行文件。
    pub(crate) fn discover(requested: Option<PathBuf>) -> Self {
        if let Some(path) = requested {
            let frozen = if path.is_absolute() {
                path.canonicalize().unwrap_or(path)
            } else {
                path
            };
            return Self::Explicit(frozen);
        }
        if let Some(path) = std::env::var_os("PATH") {
            for directory in std::env::split_paths(&path) {
                if !directory.is_absolute() {
                    continue;
                }
                let Ok(executable) = directory.join("go").canonicalize() else {
                    continue;
                };
                if std::fs::metadata(&executable).is_ok_and(|metadata| {
                    metadata.is_file() && metadata.permissions().mode() & 0o111 != 0
                }) {
                    return Self::Path(executable);
                }
            }
        }
        Self::NotFound
    }

    /// 返回本轮固定工具或显式无效请求，供原生探针报告具体阻塞。
    pub(crate) fn tool(&self) -> Option<&Path> {
        match self {
            Self::Explicit(path) | Self::Path(path) => Some(path),
            Self::NotFound => None,
        }
    }

    /// 投影选择来源和已定位路径；该信息不证明版本、工具字节或完整覆盖。
    pub(crate) fn report(&self) -> Value {
        let source = match self {
            Self::Explicit(_) => "explicit",
            Self::Path(_) => "path",
            Self::NotFound => "not_found",
        };
        let executable = self
            .tool()
            .filter(|path| path.is_absolute() && path.is_file());
        json!({"source":source, "executable":executable})
    }
}
