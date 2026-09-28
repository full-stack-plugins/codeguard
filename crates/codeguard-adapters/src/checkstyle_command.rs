use std::ffi::OsString;
use std::path::{Component, Path, PathBuf};

/// Checkstyle 自包含 JAR 的字面参数规划；不执行 JVM、不读取配置或下载工具。
pub struct CheckstyleCommand {
    /// 调用方已核验的本地自包含 JAR 绝对路径。
    pub jar: PathBuf,
    /// 原配置的受控本地快照绝对路径；不自动选内置默认规则。
    pub config: PathBuf,
    /// 本轮受控源码快照绝对路径。
    pub source: PathBuf,
    /// 本轮独立 XML 报告槽位绝对路径；新鲜性由 runtime 核对。
    pub report: PathBuf,
}

impl CheckstyleCommand {
    /// 返回传给已核验 Java 入口的固定参数；非法或互相覆盖的路径返回原因码。
    /// 不检查文件存在性、符号链接或工具身份，这些由受控执行上下文核验。
    pub fn args(&self) -> Result<Vec<OsString>, &'static str> {
        let paths = [&self.jar, &self.config, &self.source, &self.report];
        if paths.iter().any(|path| !safe_absolute(path)) {
            return Err("checkstyle_path_not_safe_absolute");
        }
        for (index, path) in paths.iter().enumerate() {
            if paths[index + 1..].contains(path) {
                return Err("checkstyle_input_output_collision");
            }
        }
        Ok(vec![
            "-jar".into(),
            self.jar.as_os_str().to_os_string(),
            "-c".into(),
            self.config.as_os_str().to_os_string(),
            "-f".into(),
            "xml".into(),
            "-o".into(),
            self.report.as_os_str().to_os_string(),
            self.source.as_os_str().to_os_string(),
        ])
    }
}

fn safe_absolute(path: &Path) -> bool {
    path.is_absolute()
        && path.components().all(|c| {
            matches!(
                c,
                Component::Prefix(_) | Component::RootDir | Component::Normal(_)
            )
        })
        && !path
            .as_os_str()
            .as_encoded_bytes()
            .iter()
            .any(|b| b.is_ascii_control())
}
