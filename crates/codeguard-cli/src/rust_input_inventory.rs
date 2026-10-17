use std::{
    collections::BTreeSet,
    io,
    path::{Path, PathBuf},
};

/// Rust 输入范围的静态清单；来源：OpenSpec execution-kernel 输入前后复核。
/// 沿用普通发现的排除和条目预算，不推断 Cargo 动态目标或外部依赖闭包。
#[derive(Eq, PartialEq)]
pub(crate) struct RustInputInventory {
    pub(crate) source_files: BTreeSet<String>,
    pub(crate) configuration_files: BTreeSet<PathBuf>,
}

impl RustInputInventory {
    /// 读取普通发现范围内的 Rust 源码、Cargo 清单与锁文件名集合。
    /// 参数为受检根；返回静态清单，读取失败或截断不能作为空清单消费。
    pub(crate) fn capture(root: &Path) -> io::Result<Self> {
        let registry = codeguard_adapters::legacy_registry()
            .map_err(|_| io::Error::other("Rust 输入注册表不可用"))?;
        let observation =
            crate::discovery::discover(root, &registry, &codeguard_runtime::NativeObservation);
        if !observation.observation_complete {
            return Err(io::Error::other("Rust 输入范围发现未完成"));
        }
        let source_files = observation
            .languages
            .get("rust")
            .map(|language| language.source_files.clone())
            .unwrap_or_default();
        let mut configuration_files = observation
            .languages
            .get("rust")
            .map(|language| language.manifests.iter().map(PathBuf::from).collect())
            .unwrap_or_else(BTreeSet::new);
        configuration_files.extend(
            observation
                .lockfiles
                .iter()
                .filter(|path| {
                    Path::new(path)
                        .file_name()
                        .is_some_and(|name| name == "Cargo.lock")
                })
                .map(PathBuf::from),
        );
        Ok(Self {
            source_files,
            configuration_files,
        })
    }
}
