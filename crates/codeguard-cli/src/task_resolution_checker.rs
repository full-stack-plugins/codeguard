use serde_json::{Value, json};
use std::{path::Path, time::Instant};

/// 限定任务服务支持的原生语法检查器；语言与协议由宿主入口固定，不读项目自选类型。
#[derive(Clone, Copy)]
pub(crate) enum TaskResolutionChecker {
    Zig,
    Erlang,
    Swift,
    Kotlin,
}

impl TaskResolutionChecker {
    /// 返回宿主入口固定的语言标识，不从策略推断语言。
    pub(crate) fn language(self) -> &'static str {
        match self {
            Self::Zig => "zig",
            Self::Erlang => "erlang",
            Self::Swift => "swift",
            Self::Kotlin => "kotlin",
        }
    }
    /// 返回允许复检的原生语法规则标识。
    pub(crate) fn rule(self) -> &'static str {
        match self {
            Self::Zig => "zig.ast_check.error",
            Self::Erlang => "erlang.syntax.error",
            Self::Swift => "swift.parse.error",
            Self::Kotlin => "kotlin.syntax",
        }
    }
    /// 返回验收范围内的固定原生版本。
    pub(crate) fn version(self) -> &'static str {
        match self {
            Self::Zig => "0.16.0",
            Self::Erlang => "OTP 28",
            Self::Swift => "Apple Swift 6.4",
            Self::Kotlin => "kotlinc-jvm 2.4.10",
        }
    }
    /// 返回该语言必须使用的批准策略版本。
    pub(crate) fn policy_version(self) -> &'static str {
        match self {
            Self::Zig => "1.0.0",
            Self::Erlang => "1.1.0",
            Self::Swift => "1.2.0",
            Self::Kotlin => "1.3.0",
        }
    }
    /// 返回该语言专用的脱敏证据版本。
    pub(crate) fn evidence_version(self) -> &'static str {
        match self {
            Self::Zig => "0.1.0",
            Self::Erlang => "0.2.0",
            Self::Swift => "0.3.0",
            Self::Kotlin => "0.4.0",
        }
    }
    /// 将指定源码字节交给固定工具，返回有界原生观察；各次调用共用截止时间。
    pub(crate) fn observe(
        self,
        tool: &Path,
        source: &[u8],
        root: &Path,
        deadline: Instant,
    ) -> Value {
        match self {
            Self::Zig => crate::zig_syntax_probe::observe(tool, source, root, deadline)
                .unwrap_or_else(|| json!({"status":"not_run","reason":"zig_tool_unavailable_or_untrusted","version":null,"tool_sha256":null,"diagnostics":[]})),
            Self::Erlang => crate::erlang_syntax_probe::observe(tool, source, deadline),
            Self::Swift => crate::swift_syntax_probe::observe(tool, source, deadline),
            Self::Kotlin => crate::kotlin_lint_command::observe(tool, source, deadline),
        }
    }
    /// 核对原反例完成状态及位置；Kotlin 同时验证 UTF-16 列与 UTF-8 字节列。
    /// 参数为原生观察和冻结反例字节；只有完整合法的观察返回 true。
    pub(crate) fn original_completed(self, native: &Value, source: &[u8]) -> bool {
        if !matches!(
            native["status"].as_str(),
            Some("completed" | "diagnostics_observed")
        ) {
            return false;
        }
        if matches!(self, Self::Kotlin) {
            return codeguard_adapters::valid_kotlin_native_observation(native, Some(source));
        }
        native["diagnostics"].as_array().is_some_and(|rows| {
            rows.iter().all(|row| {
                let line = row["line"].as_u64().unwrap_or(0) as usize;
                let column = row["column"].as_u64().unwrap_or(0) as usize;
                line > 0
                    && column > 0
                    && source
                        .split(|b| *b == b'\n')
                        .nth(line - 1)
                        .is_some_and(|bytes| column <= bytes.len() + 1)
            })
        })
    }
    /// 复检已有任务当前源码，返回原报告绑定的观察或失败原因。
    pub(crate) fn recheck(
        self,
        root: &Path,
        brief: &Value,
        tool: &Path,
        deadline: Instant,
    ) -> Result<Value, &'static str> {
        match self {
            Self::Zig => {
                crate::syntax_task_recheck::run(root, brief, Some(tool), None, None, None, deadline)
            }
            Self::Erlang => {
                crate::syntax_task_recheck::run(root, brief, None, Some(tool), None, None, deadline)
            }
            Self::Swift => {
                crate::syntax_task_recheck::run(root, brief, None, None, Some(tool), None, deadline)
            }
            Self::Kotlin => {
                crate::syntax_task_recheck::run(root, brief, None, None, None, Some(tool), deadline)
            }
        }
    }
}
