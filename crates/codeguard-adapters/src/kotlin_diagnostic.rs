use serde::Serialize;

/// Kotlin 原生诊断的脱敏定位；来源：OpenSpec syntax-precheck Kotlin 原生确认场景。
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct KotlinDiagnostic {
    /// 原生诊断类别，语法错误统一为 kotlin.syntax。
    pub rule_id: String,
    /// 一基行号。
    pub line: usize,
    /// 一基 UTF-8 字节列，由原生 UTF-16 列转换。
    pub column_byte: usize,
    /// 一基原生 UTF-16 列，保留对照依据。
    pub column_utf16: usize,
}
