use serde_json::{Value, json};
use std::{path::Path, sync::atomic::AtomicBool, time::Instant};

/// 限定任务服务支持的原生语法检查器；语言与协议由宿主入口固定，不读项目自选类型。
#[derive(Clone, Copy)]
pub(crate) enum TaskResolutionChecker {
    Zig,
    Erlang,
    Swift,
    Kotlin,
    Go,
    Rust,
}

impl TaskResolutionChecker {
    /// 返回宿主入口固定的语言标识，不从策略推断语言。
    pub(crate) fn language(self) -> &'static str {
        match self {
            Self::Zig => "zig",
            Self::Erlang => "erlang",
            Self::Swift => "swift",
            Self::Kotlin => "kotlin",
            Self::Go => "go",
            Self::Rust => "rust",
        }
    }
    /// 返回允许复检的原生语法规则标识。
    pub(crate) fn rule(self) -> &'static str {
        match self {
            Self::Zig => "zig.ast_check.error",
            Self::Erlang => "erlang.syntax.error",
            Self::Swift => "swift.parse.error",
            Self::Kotlin => "kotlin.syntax",
            Self::Go => "go.syntax",
            Self::Rust => "rust.syntax",
        }
    }
    /// 返回验收范围内的固定原生版本。
    pub(crate) fn version(self) -> &'static str {
        match self {
            Self::Zig => "0.16.0",
            Self::Erlang => "OTP 28",
            Self::Swift => "Apple Swift 6.4",
            Self::Kotlin => "kotlinc-jvm 2.4.10",
            Self::Go => "go1.23.4",
            Self::Rust => "rustfmt 1.9.0-stable",
        }
    }
    /// 返回该语言必须使用的批准策略版本。
    pub(crate) fn policy_version(self) -> &'static str {
        match self {
            Self::Zig => "1.0.0",
            Self::Erlang => "1.1.0",
            Self::Swift => "1.2.0",
            Self::Kotlin => "1.3.0",
            Self::Go => "1.6.0",
            Self::Rust => "1.7.0",
        }
    }
    /// 核对入口允许的签名策略版本；Zig 原生首次策略与旧 WASM 来源策略分开。
    pub(crate) fn accepts_policy_version(self, version: &str) -> bool {
        version == self.policy_version() || (matches!(self, Self::Zig) && version == "1.4.0")
    }
    /// 返回该语言专用的脱敏证据版本；参数保留 Zig 原生首次来源与旧来源的区别。
    pub(crate) fn evidence_version(self, policy_version: &str) -> &'static str {
        match self {
            Self::Zig if policy_version == "1.4.0" => "0.5.0",
            Self::Zig => "0.1.0",
            Self::Erlang => "0.2.0",
            Self::Swift => "0.3.0",
            Self::Kotlin => "0.4.0",
            Self::Go => "0.7.0",
            Self::Rust => "0.8.0",
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
        self.observe_with_cancellation(tool, source, root, deadline, &AtomicBool::new(false))
    }
    /// 传递同一请求取消令牌，不在原生观察器中创建独立令牌。
    pub(crate) fn observe_with_cancellation(
        self,
        tool: &Path,
        source: &[u8],
        root: &Path,
        deadline: Instant,
        cancelled: &AtomicBool,
    ) -> Value {
        match self {
            Self::Zig => crate::zig_syntax_probe::observe_with_cancellation(tool, source, root, deadline, cancelled)
                .unwrap_or_else(|| json!({"status":"not_run","reason":"zig_tool_unavailable_or_untrusted","version":null,"tool_sha256":null,"diagnostics":[]})),
            Self::Erlang => crate::erlang_syntax_probe::observe_with_cancellation(tool, source, deadline, cancelled),
            Self::Swift => crate::swift_syntax_probe::observe_with_cancellation(tool, source, deadline, cancelled),
            Self::Kotlin => crate::kotlin_lint_command::observe_with_cancellation(tool, source, deadline, cancelled),
            Self::Go => crate::go_syntax_probe::observe(tool, source, deadline, cancelled),
            Self::Rust => json!({"status":"incomplete","reason":"rust_edition_context_unresolved"}),
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
        if matches!(self, Self::Rust) {
            return crate::rust_syntax_evidence::valid(native, Some(source));
        }
        if matches!(self, Self::Go) {
            return crate::go_syntax_probe::valid_observation(native, Some(source));
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
            Self::Go => crate::syntax_task_recheck::run_go(root, brief, Some(tool), deadline),
            Self::Rust => crate::rust_syntax_task_recheck::run(root, brief, Some(tool), deadline),
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
