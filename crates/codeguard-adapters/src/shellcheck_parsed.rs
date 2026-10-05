use crate::shellcheck_diagnostic::ShellCheckDiagnostic;
/// ShellCheck 固定版本 json1 的局部事实；报告有效不等于策略或项目覆盖通过。
#[derive(Clone, Debug)]
pub struct ShellCheckParsed {
    /// JSON 形状及原生位置都已核对。
    pub report_valid: bool,
    /// 本轮原生退出与完整报告一致且无方言/源依赖环境阻塞。
    pub local_scan_complete: bool,
    /// 有效部分源码诊断，不能因为兄弟记录故障丢弃。
    pub diagnostics: Vec<ShellCheckDiagnostic>,
    /// 方言、配置或源依赖环境诊断，不能制造源码违规。
    pub environment_codes: Vec<String>,
    /// 固定未完成原因。
    pub reason: Option<&'static str>,
}
