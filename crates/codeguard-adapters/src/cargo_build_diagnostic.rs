//! Cargo 原生编译错误的有界结构化定位，不携带自由文本指令。

/// 一条编译错误观察；执行层还须核对原项目、源字节及目标归属。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CargoBuildDiagnostic {
    /// Rust 编译器原生错误编码。
    pub code: String,
    /// 原生主定位文件，不在解析层推定为受检源码。
    pub path: String,
    /// 原生起始行。
    pub line: u64,
    /// 原生起始列。
    pub column: u64,
    /// 原生 UTF8 字节起点。
    pub byte_start: u64,
    /// 原生字节终点，不包含该字节。
    pub byte_end: u64,
    /// 原生包身份，非可信批准。
    pub package_id: String,
    /// 原生清单路径。
    pub manifest_path: String,
    /// 原生目标源路径。
    pub target_source: String,
    /// 原生目标类别，不能替代完整构建组合覆盖。
    pub target_kinds: Vec<String>,
}
