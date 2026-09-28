use crate::cargo_build_diagnostic::CargoBuildDiagnostic;

/// Cargo 构建机器流观察；无 issue 只证明本次机器流与退出契约一致。
#[derive(Debug)]
pub struct CargoBuildParsed {
    /// 原生编译错误；出现 issue 后仅作调查证据。
    pub diagnostics: Vec<CargoBuildDiagnostic>,
    /// 原生结束事件是否成功，未观察到时为空。
    pub build_success: Option<bool>,
    /// 无法解释或不一致的协议原因。
    pub issue: Option<&'static str>,
}
