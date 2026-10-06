//! 有界Gradle原生模型采集请求。
use std::{collections::BTreeSet, path::PathBuf, time::Instant};
/// 调用方显式提供选定项目文件、已有工具和截止时间；不代表完整构建范围。
pub struct Request {
    /// 当前受检构建根。
    pub project_root: PathBuf,
    /// 相对构建根的选定输入，必须包含根settings及build脚本。
    pub project_files: BTreeSet<PathBuf>,
    /// 已安装Gradle分发目录，使用bin/gradle并核对整个目录字节。
    pub gradle_bundle: PathBuf,
    /// 已安装JDK目录，核对java入口和release；完整JDK尚未资格验收。
    pub java_home: PathBuf,
    /// 统一执行截止时间。
    pub deadline: Instant,
}
