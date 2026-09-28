use serde::{Deserialize, Serialize};

/// 单个已选源码文件的语法初检观察；不表示原生 lint 或编译结果。
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "status", rename_all = "snake_case", deny_unknown_fields)]
pub enum SyntaxFileState {
    /// 已解析；grammar_qualified 仅表示该文件的版本与方言已被验收。
    Checked {
        /// ERROR/MISSING 原始恢复节点数，尚未确认是否为源码违规。
        recoveries: usize,
        /// 该文件版本与方言是否落在已验收范围内。
        grammar_qualified: bool,
        /// 恢复节点或遍历预算是否截断。
        truncated: bool,
    },
    /// 解析、位置映射、预算或输入身份等未能完成。
    Incomplete {
        /// 具体失败原因。
        reason: String,
    },
    /// 不支持的文件、语言区域或方言。
    Unsupported {
        /// 具体不支持原因。
        reason: String,
    },
}
