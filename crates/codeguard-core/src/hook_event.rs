use serde::Serialize;

/// 宿主报告的触发阶段；它本身不是可信代码或 Git 快照。
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum HookEvent {
    /// 会话启动或恢复。
    SessionStart,
    /// 用户提交提示；仅供非阻断性意图提示。
    PromptSubmitted,
    /// 文件编辑工具完成后。
    FileChanged,
    /// 智能体完成一次修复尝试后。
    RepairReady,
    /// 提交前。
    PreCommit,
    /// 推送前。
    PrePush,
    /// 持续集成或正式交付检查。
    Ci,
    /// 会话结束。
    Stop,
}
