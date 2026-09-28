//! PMD 6 原生命令参数规划；只构造字面 argv，不执行工具或解释规则。

use std::ffi::OsString;
use std::path::{Component, Path, PathBuf};

/// Unix `run.sh pmd` 扫描单个 Java 源文件的原生命令输入。
pub struct Pmd6Command {
    /// 已由调用方确定和核验的绝对源码路径。
    pub source: PathBuf,
    /// 已锁定制品中的单一 PMD 规则集资源引用。
    pub ruleset: String,
    /// 本次私有运行区内的绝对 XML 报告路径。
    pub report: PathBuf,
}

impl Pmd6Command {
    /// 生成 PMD 6 `run.sh` 的固定字面参数，报告路径仅来自 `report`。
    pub fn args(&self) -> Result<Vec<OsString>, &'static str> {
        if !cfg!(unix) {
            return Err("pmd6_unix_launcher_only");
        }
        if !safe_absolute(&self.source) || !safe_absolute(&self.report) {
            return Err("pmd_path_not_safe_absolute");
        }
        if self.source == self.report {
            return Err("pmd_source_is_report");
        }
        if self.ruleset.trim() != self.ruleset
            || self.ruleset.is_empty()
            || self.ruleset.starts_with('-')
            || self.ruleset.contains(',')
            || self.ruleset.contains("://")
        {
            return Err("pmd_ruleset_reference_invalid");
        }
        Ok(vec![
            "pmd".into(),
            "-d".into(),
            self.source.as_os_str().to_os_string(),
            "-R".into(),
            self.ruleset.as_str().into(),
            "-f".into(),
            "xml".into(),
            "-r".into(),
            self.report.as_os_str().to_os_string(),
            "-showsuppressed".into(),
            "-no-cache".into(),
        ])
    }
}

fn safe_absolute(path: &Path) -> bool {
    path.is_absolute()
        && path
            .components()
            .all(|component| matches!(component, Component::RootDir | Component::Normal(_)))
}
