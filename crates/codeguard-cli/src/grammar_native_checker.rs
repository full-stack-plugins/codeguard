use crate::task_resolution_checker::TaskResolutionChecker;
use serde_json::Value;
use std::{path::Path, sync::atomic::AtomicBool, time::Instant};

/// 开发期语法差分工具选择；扩展回放不扩大可信任务关闭服务的授权范围。
#[derive(Clone, Copy)]
pub(crate) enum GrammarNativeChecker {
    /// 已有产品原生语法观察器。
    Existing(TaskResolutionChecker),
    /// Ruff 隔离语法观察，仅对固定 Python 3.12 语法作开发对照。
    Python,
    /// Node 的显式 ESM 语法观察，不推断项目模块类型。
    Javascript,
}

impl GrammarNativeChecker {
    /// 解析显式语言标识；未知语言不执行进程。
    pub(crate) fn for_language(language: &str) -> Option<Self> {
        match language {
            "zig" => Some(Self::Existing(TaskResolutionChecker::Zig)),
            "erlang" => Some(Self::Existing(TaskResolutionChecker::Erlang)),
            "swift" => Some(Self::Existing(TaskResolutionChecker::Swift)),
            "kotlin" => Some(Self::Existing(TaskResolutionChecker::Kotlin)),
            "python" => Some(Self::Python),
            "javascript" => Some(Self::Javascript),
            _ => None,
        }
    }
    /// 返回本次允许的原生版本，不从项目脚本推断。
    pub(crate) fn version(self) -> &'static str {
        match self {
            Self::Existing(checker) => checker.version(),
            Self::Python => "ruff 0.16.8",
            Self::Javascript => "v24.18.0",
        }
    }
    /// 返回对应原生制品的有界字节预算；Node 独立预算不扩张其它工具权限。
    pub(crate) fn artifact_budget(self) -> u64 {
        match self {
            Self::Javascript => crate::javascript_syntax_probe::NODE_ARTIFACT_BUDGET,
            _ => 64 * 1024 * 1024,
        }
    }
    /// 对冻结源码执行隔离语法观察；工具、源码与预算由差分入口绑定。
    pub(crate) fn observe(
        self,
        tool: &Path,
        source: &[u8],
        root: &Path,
        deadline: Instant,
        cancelled: &AtomicBool,
    ) -> Value {
        match self {
            Self::Existing(checker) => checker.observe(tool, source, root, deadline),
            Self::Python => crate::python_syntax_probe::observe(tool, source, deadline),
            Self::Javascript => {
                crate::javascript_syntax_probe::observe(tool, source, deadline, cancelled)
            }
        }
    }
}
