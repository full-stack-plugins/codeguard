use crate::ResolutionCause;
/// 一项任务的关闭判定；不包含项目交付许可。
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ResolutionOutcome {
    /// 完整证据支持限定任务解决。
    Resolved(ResolutionCause),
    /// 原问题仍存在。
    StillOpen,
    /// 同字节原生反证应进入误报调查，不统计为代码修复。
    FalsePositiveReviewRequired,
    /// 证据不完整或归因尚无适用验证器。
    Incomplete(&'static str),
}
