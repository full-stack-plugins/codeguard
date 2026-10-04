use crate::KotlinDiagnostic;

/// 单一冻结输入的 Kotlin 语法与上下文诊断；来源：OpenSpec Kotlin 原生确认。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct KotlinParsed {
    /// 原生 SYNTAX 诊断，精确定位归并，不保留源文本和错误文案。
    pub syntax: Vec<KotlinDiagnostic>,
    /// 类型、依赖等上下文诊断，不冒充语法错误。
    pub context: Vec<KotlinDiagnostic>,
}
