/// kotlinc-jvm 2.4.10 限定语法任务输入；宿主独立提供原反例、工具、签名及可信上下文。
/// 单文件上下文不完整不能关闭任务；此输入不批准全项目 lint 或类型检查。
pub type KotlinTaskResolutionRequest<'a> =
    crate::syntax_task_resolution_request::SyntaxTaskResolutionRequest<'a>;
