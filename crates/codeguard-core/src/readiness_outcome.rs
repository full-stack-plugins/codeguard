//! 前置汇总与具体阻塞/未知标识；不含交付 allow。
use crate::ReadinessState;
use serde::{Deserialize, Serialize};
/// 宿主可以据此规划准备任务，不能用此结果关闭源码 finding。
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReadinessOutcome {
    /// 前置准备状态。
    pub state: ReadinessState,
    /// 当前已确认必需阻塞，按稳定标识去重排序。
    pub blocked_ids: Vec<String>,
    /// 尚需解析/探测/刷新或身份复核的必需条件。
    pub unresolved_ids: Vec<String>,
    /// 结构化诊断码，不含原生文本、环境变量或执行命令。
    pub reasons: Vec<String>,
}
