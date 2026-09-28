//! 原生 rustdoc 诊断的结构化观察，不保留报告中的自由文本指令。

/// 一条原生文档规则观察；目标归属须由执行层另行核验。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RustdocFinding {
    /// 原生规则 ID。
    pub rule_id: String,
    /// 原生主定位文件。
    pub path: String,
    /// 原生主定位起始行，必须非零。
    pub line: u64,
    /// 原生主定位起始列，必须非零。
    pub column: u64,
    /// 原生主定位字节范围起点；不得由行号猜测。
    pub byte_start: u64,
    /// 原生主定位字节范围终点，不包含该字节。
    pub byte_end: u64,
    /// 原生级别 warning 或 error。
    pub level: String,
    /// Cargo 原生包身份；不是可信批准。
    pub package_id: String,
    /// Cargo 原生清单路径，供执行层绑定当前输入。
    pub manifest_path: String,
    /// Cargo 原生目标源码路径。
    pub target_source: String,
    /// 原生目标类别，不推断全部构建组合已检查。
    pub target_kinds: Vec<String>,
}
