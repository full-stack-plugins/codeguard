/// Zig 限定语法任务请求；与公共原生关闭服务共享输入契约，原有构造方式保持兼容。
pub type ZigTaskResolutionRequest<'a> =
    crate::syntax_task_resolution_request::SyntaxTaskResolutionRequest<'a>;
