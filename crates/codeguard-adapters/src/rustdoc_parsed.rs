//! Cargo/rustdoc JSON 流的观察状态。

use crate::rustdoc_finding::RustdocFinding;

/// 解析结果；无 issue 和原生成功终结均成立才可视为协议完整。
#[derive(Debug)]
pub struct RustdocParsed {
    /// 已保留的原生文档观察；出现 issue 时仅供调查。
    pub findings: Vec<RustdocFinding>,
    /// 是否观察到原生成功终结事件；不代表进程退出或项目覆盖有效。
    pub build_finished: bool,
    /// 机器流不能完整解释的原因。
    pub issue: Option<&'static str>,
}
