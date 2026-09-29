use serde::{Deserialize, Serialize};

/// 原生能力缺失时给修复工作台的准备动作；不代表质量或交付判定。
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SyntaxSetupAction {
    /// 可选原生工具缺失且完整初检正常；仅显示一次建议。
    RecommendNativeTool,
    /// 必须准备原生工具并核对当前源码，不能仅凭安装关闭任务。
    RequireNativeToolAndConfirmation,
    /// 修复已有工具的无效配置，再执行适用原生确认。
    RepairNativeConfigurationAndConfirm,
    /// 恢复失败的原生执行并复检，保留已有局部原生结果。
    RecoverNativeExecutionAndConfirm,
    /// 没有可用适配器，需要明确能力和验证路径决策。
    RequireCapabilityDecision,
}
