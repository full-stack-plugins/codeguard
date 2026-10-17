/// 私有worker请求范围；模块规则必须由显式Module请求启用。
#[derive(Clone, Copy, Eq, PartialEq)]
pub(crate) enum SyntaxWorkerMode {
    /// 仅既有通用候选。
    Default,
    /// 显式语言结构规则，不推断模块模式。
    Bindings,
    /// 已请求JavaScript module上下文。
    Module,
}
