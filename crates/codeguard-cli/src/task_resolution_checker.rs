use serde_json::{Value, json};
use std::{path::Path, time::Instant};

/// 限定任务服务支持的原生语法检查器；语言与协议由宿主入口固定，不读项目自选类型。
#[derive(Clone, Copy)]
pub(crate) enum TaskResolutionChecker {
    Zig,
    Erlang,
}

impl TaskResolutionChecker {
    /// 返回宿主入口固定的语言标识，不从策略推断语言。
    pub(crate) fn language(self) -> &'static str {
        match self {
            Self::Zig => "zig",
            Self::Erlang => "erlang",
        }
    }
    /// 返回允许复检的原生语法规则标识。
    pub(crate) fn rule(self) -> &'static str {
        match self {
            Self::Zig => "zig.ast_check.error",
            Self::Erlang => "erlang.syntax.error",
        }
    }
    /// 返回验收范围内的固定原生版本。
    pub(crate) fn version(self) -> &'static str {
        match self {
            Self::Zig => "0.16.0",
            Self::Erlang => "OTP 28",
        }
    }
    /// 返回该语言必须使用的批准策略版本。
    pub(crate) fn policy_version(self) -> &'static str {
        match self {
            Self::Zig => "1.0.0",
            Self::Erlang => "1.1.0",
        }
    }
    /// 返回该语言专用的脱敏证据版本。
    pub(crate) fn evidence_version(self) -> &'static str {
        match self {
            Self::Zig => "0.1.0",
            Self::Erlang => "0.2.0",
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
        }
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
            Self::Zig => crate::syntax_task_recheck::run(root, brief, Some(tool), None, deadline),
            Self::Erlang => {
                crate::syntax_task_recheck::run(root, brief, None, Some(tool), deadline)
            }
        }
    }
}
