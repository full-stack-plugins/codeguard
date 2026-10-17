//! 有界Gradle/JDK Javadoc原生位置，不包含自由文本或源码内容。
use serde::Serialize;
/// 原生JDK缺注释诊断，路径仍须由调用方映射到当前输入快照。
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct GradleJavadocDiagnostic {
    /// 本轮私有源码绝对身份，公开前转换为选定相对路径。
    pub path: String,
    /// 已识别的JDK缺注释规则。
    pub rule_id: &'static str,
    /// 原生一基行。
    pub line: u32,
    /// 原生一基caret列。
    pub column: u32,
}
