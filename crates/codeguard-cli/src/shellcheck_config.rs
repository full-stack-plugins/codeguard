//! ShellCheck 配置的有界静态快照；保留最近缺项，绝不执行配置或允许外部 source 读取。
use codeguard_runtime::read_bounded_regular_file;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    io::ErrorKind,
    path::{Path, PathBuf},
};
type ConfigInputs = Vec<(PathBuf, Option<Vec<u8>>)>;
/// ShellCheck 最近配置与更近目录缺项的原始观察，用于运行前后复核。
#[derive(Clone, Eq, PartialEq)]
pub(crate) struct ShellCheckConfig {
    requested: Option<PathBuf>,
    inputs: ConfigInputs,
    selected: Option<(PathBuf, Vec<u8>)>,
}
impl ShellCheckConfig {
    /// 读取显式配置或最多64层祖先的首个原生rc；返回快照或具体环境原因。
    pub fn capture(source: &Path, requested: Option<&Path>) -> Result<Self, &'static str> {
        let mut out = Self {
            requested: requested.map(Path::to_path_buf),
            inputs: Vec::new(),
            selected: None,
        };
        let paths: Vec<PathBuf> = if let Some(path) = requested {
            vec![path.to_path_buf()]
        } else {
            source
                .parent()
                .ok_or("shellcheck_config_unresolved")?
                .ancestors()
                .take(64)
                .flat_map(|p| [p.join(".shellcheckrc"), p.join("shellcheckrc")])
                .collect()
        };
        for path in paths {
            match std::fs::symlink_metadata(&path) {
                Err(e) if e.kind() == ErrorKind::NotFound && requested.is_none() => {
                    out.inputs.push((path, None))
                }
                Err(_) => return Err("shellcheck_config_unreadable"),
                Ok(_) => {
                    let bytes = read_bounded_regular_file(&path, 32 * 1024)
                        .map_err(|_| "shellcheck_config_unreadable")?;
                    let text = std::str::from_utf8(&bytes)
                        .map_err(|_| "shellcheck_config_invalid_encoding")?;
                    for line in text.lines() {
                        let line = line.split('#').next().unwrap_or("").trim();
                        if line.starts_with("external-sources")
                            && !line.split_once('=').is_some_and(|(key, value)| {
                                key.trim() == "external-sources" && value.trim() == "false"
                            })
                        {
                            return Err("shellcheck_external_sources_unverified");
                        }
                    }
                    out.inputs.push((path.clone(), Some(bytes.clone())));
                    out.selected = Some((path, bytes));
                    break;
                }
            }
        }
        Ok(out)
    }
    /// 按原始输入重新读取，检测新增配置、字节变化及链接替换。
    pub fn current(&self, source: &Path) -> bool {
        Self::capture(source, self.requested.as_deref())
            .ok()
            .as_ref()
            == Some(self)
    }
    /// 返回原始配置字节；调用者须写入本轮0700私有运行区。
    pub fn bytes(&self) -> Option<&[u8]> {
        self.selected.as_ref().map(|(_, bytes)| bytes.as_slice())
    }
    /// 返回配置检测反馈；已配置不意味着策略获得批准。
    pub fn report(&self) -> Value {
        json!({"status":if self.selected.is_some(){"configured"}else{"missing"},"source_path":self.selected.as_ref().map(|(path,_)|path),"sha256":self.bytes().map(|bytes|format!("{:x}",Sha256::digest(bytes))),"reason":null,"global_configuration":"not_loaded"})
    }
}
