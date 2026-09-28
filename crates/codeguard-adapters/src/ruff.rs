//! Ruff 机器报告的纯解析；进程执行、版本锁与内容身份由其它边界提供。

use serde::Deserialize;

/// 判断精确 Ruff pydocstyle 规则 ID；仅用于原生诊断归类，不代表规则存在、启用或获批。
#[must_use]
pub fn is_ruff_pydocstyle_rule(code: &str) -> bool {
    let bytes = code.as_bytes();
    bytes.len() == 4 && bytes[0] == b'D' && bytes[1..].iter().all(u8::is_ascii_digit)
}

/// Ruff 报告中的一处行列位置。
#[derive(Clone, Debug, Eq, PartialEq, Deserialize)]
pub struct RuffLocation {
    /// 一基行号。
    pub row: u32,
    /// 一基列号。
    pub column: u32,
}

/// Ruff 原生诊断，保留原始规则和严重度供批准策略独立判定。
#[derive(Clone, Debug, Eq, PartialEq, Deserialize)]
pub struct RuffDiagnostic {
    /// Ruff 原生规则 ID，例如 F401。
    pub code: String,
    /// 原生诊断文字；对外报告前仍需脱敏。
    pub message: String,
    /// 原生路径；由执行上下文比对并转换为可逆项目路径。
    pub filename: String,
    /// 起始位置。
    pub location: RuffLocation,
    /// 原生严重度，不在解析阶段猜测门禁阈值。
    pub severity: String,
}

/// Ruff JSON 与退出码组合的有效性。
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RuffParseState {
    /// 原生报告结构与受支持的退出语义一致。
    Valid,
    /// 工具故障、报告损坏或内部矛盾；有效发现仍保留。
    Incomplete,
}

/// 原生解析产物；不能单独证明内容、规则、工具或扫描覆盖。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RuffParsed {
    /// 报告与退出语义的判定。
    pub state: RuffParseState,
    /// 在完整 JSON 数组中逐条验证后保留的有效诊断。
    pub diagnostics: Vec<RuffDiagnostic>,
    /// 原生工具报告的读取故障；不得转换为源码违规。
    pub environment_diagnostics: Vec<RuffDiagnostic>,
    /// 未完成原因，供运行报告关联原始私有证据。
    pub reason: Option<&'static str>,
}

/// 解析 Ruff JSON 机器报告；只解释原生契约，不执行命令或判定质量策略。
#[must_use]
pub fn parse_ruff_json(native_exit_code: i32, stdout: &[u8]) -> RuffParsed {
    const MAX_REPORT_BYTES: usize = 16 * 1024 * 1024;
    if stdout.len() > MAX_REPORT_BYTES {
        return incomplete(Vec::new(), Vec::new(), "report_too_large");
    }
    let Ok(value) = serde_json::from_slice::<serde_json::Value>(stdout) else {
        return incomplete(Vec::new(), Vec::new(), "invalid_json_report");
    };
    let Some(items) = value.as_array() else {
        return incomplete(Vec::new(), Vec::new(), "invalid_report_root");
    };
    let mut diagnostics = Vec::with_capacity(items.len());
    let mut environment_diagnostics = Vec::new();
    let mut invalid_entry = false;
    for item in items {
        match serde_json::from_value::<RuffDiagnostic>(item.clone()) {
            Ok(diagnostic)
                if !diagnostic.code.is_empty()
                    && !diagnostic.message.is_empty()
                    && !diagnostic.filename.is_empty()
                    && !diagnostic.severity.is_empty()
                    && diagnostic.location.row > 0
                    && diagnostic.location.column > 0 =>
            {
                if diagnostic.code == "E902" {
                    environment_diagnostics.push(diagnostic);
                } else {
                    diagnostics.push(diagnostic);
                }
            }
            _ => invalid_entry = true,
        }
    }
    if invalid_entry {
        return incomplete(diagnostics, environment_diagnostics, "invalid_report_entry");
    }
    if !environment_diagnostics.is_empty() {
        return incomplete(diagnostics, environment_diagnostics, "native_io_error");
    }
    match (native_exit_code, diagnostics.is_empty()) {
        (0, true) | (1, false) => RuffParsed {
            state: RuffParseState::Valid,
            diagnostics,
            environment_diagnostics,
            reason: None,
        },
        (0 | 1, _) => incomplete(diagnostics, environment_diagnostics, "exit_report_mismatch"),
        _ => incomplete(diagnostics, environment_diagnostics, "native_tool_failure"),
    }
}

fn incomplete(
    diagnostics: Vec<RuffDiagnostic>,
    environment_diagnostics: Vec<RuffDiagnostic>,
    reason: &'static str,
) -> RuffParsed {
    RuffParsed {
        state: RuffParseState::Incomplete,
        diagnostics,
        environment_diagnostics,
        reason: Some(reason),
    }
}
