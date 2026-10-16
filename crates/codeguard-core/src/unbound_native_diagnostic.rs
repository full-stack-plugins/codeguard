/// 尚未与独立本次源码/原生日志绑定的类型化诊断，不能进入已确认Findings或VIOLATION。无Java对应类型。
#[derive(Clone, Debug)]
pub struct UnboundNativeDiagnostic {
    pub checker_id: String,
    pub native_rule_id: String,
    pub path: String,
    pub line: Option<u32>,
    pub column: Option<u32>,
    pub severity: String,
    pub reason: String,
}
