use serde::Deserialize;

/// 开发期固定语法回归样本；标签不具独立验收或策略批准权威。
/// 来源：OpenSpec syntax-precheck 全量开发回放场景。
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GrammarEvaluationCase {
    /// 语料范围内唯一的样本标识。
    pub id: String,
    /// 当前清单中的准确语言标识。
    pub language: String,
    /// UTF-8 源码字节，不作为宿主执行指令。
    pub source: String,
    /// 固定源码的 SHA-256。
    pub source_sha256: String,
    /// 回归标签或待裁定的临时预期。
    pub expected_valid: bool,
    /// regression 或 pending；不接收自行声明的批准 holdout。
    pub label: String,
    /// 可供复核的仓库内来源说明。
    pub origin: String,
}
