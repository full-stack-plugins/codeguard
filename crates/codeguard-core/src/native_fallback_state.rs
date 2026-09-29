use serde::{Deserialize, Serialize};

/// 原生语法确认能力暂不可执行时的具体原因；由可信发现与执行结果提供。
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum NativeFallbackState {
    /// 有适用适配器，但尚未找到所需工具。
    MissingTool,
    /// 工具存在而配置无效；不得再建议重复安装。
    InvalidConfiguration,
    /// 工具或配置已定位，但原生执行未完成。
    ExecutionFailed,
    /// 没有适用的原生确认适配器。
    NoAdapter,
}
