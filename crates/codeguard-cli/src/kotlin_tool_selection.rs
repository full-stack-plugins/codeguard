use serde_json::{Value, json};
use std::{
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
};

/// 一次性选择 Kotlin 编译入口；来源：OpenSpec 原生优先与禁止故障换工具契约。
pub(crate) enum KotlinToolSelection {
    Explicit(PathBuf),
    Path(PathBuf),
    NotFound,
}
impl KotlinToolSelection {
    /// 显式路径优先；否则只从绝对 PATH 目录选择首个普通可执行入口，不启动或安装工具。
    pub(crate) fn discover(requested: Option<PathBuf>) -> Self {
        if let Some(path) = requested {
            return Self::Explicit(path);
        }
        if let Some(path) = std::env::var_os("PATH") {
            for directory in std::env::split_paths(&path) {
                if !directory.is_absolute() {
                    continue;
                }
                let Ok(directory) = directory.canonicalize() else {
                    continue;
                };
                let entry = directory.join("kotlinc");
                if std::fs::metadata(&entry)
                    .is_ok_and(|m| m.is_file() && m.permissions().mode() & 0o111 != 0)
                {
                    return Self::Path(entry);
                }
            }
        }
        Self::NotFound
    }
    /// 保留选定入口身份；版本或运行失败后不得切换到其它工具或 WASM。
    pub(crate) fn tool(&self) -> Option<&Path> {
        match self {
            Self::Explicit(path) | Self::Path(path) => Some(path),
            Self::NotFound => None,
        }
    }
    /// 输出选择来源及入口，不授予整个编译器/JDK 工具链的可信身份。
    pub(crate) fn report(&self) -> Value {
        json!({"source":match self {Self::Explicit(_)=>"explicit",Self::Path(_)=>"path",Self::NotFound=>"not_found"},"executable":self.tool()})
    }
}
