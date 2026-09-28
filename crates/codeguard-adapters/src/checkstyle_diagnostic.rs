/// Checkstyle 原生诊断；对应 XMLLogger 的 error 事件，不代表受批准规则或项目归属。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CheckstyleDiagnostic {
    /// 原生路径，公开前须绑定项目范围并脱敏。
    pub filename: String,
    /// 完整原生 source，包括自定义模块 ID；禁止只按类名合并。
    pub source: String,
    /// 原生行号；零表示文件级诊断，没有可推导的源码行。
    pub line: u32,
    /// 原生可选的一基列号。
    pub column: Option<u32>,
    /// 原生 error/warning/info，不自行升级为门禁严重度。
    pub severity: String,
    /// 原生消息，公开前必须脱敏。
    pub message: String,
}
