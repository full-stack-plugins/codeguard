/// Apple Swift 6.4 限定 parse 任务输入；宿主独立提供策略、信任根、原反例及截止时间。
/// 只覆盖原生语法复检，不证明 SwiftLint、类型检查或项目构建完成。
pub type SwiftTaskResolutionRequest<'a> =
    crate::syntax_task_resolution_request::SyntaxTaskResolutionRequest<'a>;
