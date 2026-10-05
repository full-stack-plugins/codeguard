/// 受保护宿主提供的 Rust 原生语法任务关闭请求；来源：OpenSpec 修复闭环契约。
/// 原反例、工具、密钥与策略上下文必须由宿主独立固定，不从项目获取批准。
pub type RustTaskResolutionRequest<'a> =
    crate::syntax_task_resolution_request::SyntaxTaskResolutionRequest<'a>;
