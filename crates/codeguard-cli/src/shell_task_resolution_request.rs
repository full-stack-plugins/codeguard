/// ShellCheck原工具限定修复请求；来源：OpenSpec原规则修复闭环。
/// 信任根和原样本由独立宿主提供，不能从项目批准文件选取。
pub type ShellTaskResolutionRequest<'a> =
    crate::syntax_task_resolution_request::SyntaxTaskResolutionRequest<'a>;
