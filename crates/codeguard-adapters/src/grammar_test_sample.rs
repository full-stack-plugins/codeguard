/// 上游 grammar corpus 的原字节样本；仅供开发回归，不是独立原生 oracle。
/// 来源：OpenSpec syntax-precheck 上游语料导入契约。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GrammarTestSample {
    /// corpus 标题，保持可追溯名称。
    pub name: String,
    /// 分隔符之间的原 UTF-8 源码，包括原始换行。
    pub source: String,
    /// 上游预期树是否明确含 ERROR 或 MISSING 节点。
    pub expected_error: bool,
}
