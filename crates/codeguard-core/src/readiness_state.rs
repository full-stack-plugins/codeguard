//! 前置准备状态，与质量门禁决策分离。
use serde::{Deserialize, Serialize};
/// 前置已准备、已确认阻塞或尚无完整证据；ready 不代表检查通过。
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReadinessState {
    /// 适用必需集合完整且全部具有本轮有效满足证据。
    Ready,
    /// 当前必需前置存在已确认缺失、不兼容或冲突。
    Incomplete,
    /// 来源、集合、适用性或当前证据仍不完整。
    Unknown,
}
