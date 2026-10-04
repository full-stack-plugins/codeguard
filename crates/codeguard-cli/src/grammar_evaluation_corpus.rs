use crate::grammar_evaluation_case::GrammarEvaluationCase;
use serde::Deserialize;

/// 绑定固定 grammar 清单的开发期语料，不是独立 holdout。
/// 来源：OpenSpec S12.11 / S14.17 开发评测契约。
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GrammarEvaluationCorpus {
    /// 当前唯一支持的语料协议版本。
    pub schema_version: String,
    /// grammar_regression 明确限制此入口的用途。
    pub corpus_type: String,
    /// 冻结的随仓 grammar 清单摘要。
    pub manifest_sha256: String,
    /// 需全部回放的固定样本，失败样本不得删去提高指标。
    pub cases: Vec<GrammarEvaluationCase>,
}
