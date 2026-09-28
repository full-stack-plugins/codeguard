use serde::Serialize;

/// 一次事件应请求的检查阶段；具体检查器由正式 CheckPlan 决定。
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum HookTriggerAction {
    /// 已确认写入失败，无须启动源码检查。
    NoCheck,
    /// 只读观察项目语言与配置。
    DiscoverProject,
    /// 根据用户意图展示非阻断性检查建议，不运行 Git 门禁。
    ShowIntentGuidance,
    /// 对明确编辑且确认写入的文件提供快速反馈。
    FastFileCheck,
    /// 无可靠编辑目标时重新确定范围，不能称为已检查。
    ResolveChangedScope,
    /// 按任务身份运行原检查器复检。
    VerifyTask,
    /// 对本轮 Git index 内容面运行严格提交门禁。
    CommitGate,
    /// 对本轮实际推送 ref 内容面运行严格推送门禁。
    PushGate,
    /// 执行完整项目义务。
    FullProjectCheck,
    /// 仅汇总本次会话状态。
    ShowSummary,
}
