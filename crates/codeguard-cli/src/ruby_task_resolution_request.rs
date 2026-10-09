/// 受保护宿主提供的 Ruby 原工具任务关闭请求；来源：OpenSpec 修复闭环契约。
/// 参数绑定固定工具、原样本及独立宿主策略；不授予项目自批准能力。
pub type RubyTaskResolutionRequest<'a> =
    crate::syntax_task_resolution_request::SyntaxTaskResolutionRequest<'a>;
