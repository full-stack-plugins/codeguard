use serde::{Deserialize, Serialize};

/// 独立于原始ERROR/MISSING的Codeguard结构规则观察；只要求原生确认，不赋予违规权威。
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SyntaxWorkerStructure {
    /// 固定来源类型，不冒充解析器恢复。
    pub basis: String,
    /// 内置规则标识。
    pub rule_id: String,
    /// 内置规则版本。
    pub rule_version: String,
    /// 固定规则配置字节摘要。
    pub rule_sha256: String,
    /// 规则对应语法节点类型；Python为block父节点，Go为整文件根节点。
    pub parent_syntax_kind: String,
    /// 原始字节起点。
    pub start_byte: usize,
    /// 原始字节终点。
    pub end_byte: usize,
    /// 零基起始行。
    pub start_row: usize,
    /// 零基起始字节列。
    pub start_column_byte: usize,
    /// 零基结束行。
    pub end_row: usize,
    /// 零基结束字节列。
    pub end_column_byte: usize,
}

impl SyntaxWorkerStructure {
    /// 核对固定规则、父节点和原始源码位置；参数必须是同轮冻结源码与语言。
    pub fn valid(&self, language: &str, source: &[u8]) -> bool {
        let rule_valid = match language {
            "python" => {
                self.rule_id == "codeguard.python.required_suite"
                    && self.rule_sha256 == codeguard_adapters::python_suite_rule_sha256()
                    && codeguard_adapters::is_required_python_suite_parent(&self.parent_syntax_kind)
            }
            "go" => {
                self.rule_id == "codeguard.go.required_package"
                    && self.rule_sha256 == codeguard_adapters::go_package_rule_sha256()
                    && self.parent_syntax_kind == "source_file"
                    && self.start_byte == 0
                    && self.end_byte == 0
            }
            "cfquery" => {
                self.rule_id == "codeguard.cfquery.distinct_projection"
                    && self.rule_sha256 == codeguard_adapters::cfquery_projection_rule_sha256()
                    && self.parent_syntax_kind == "program"
                    && source
                        .get(self.start_byte..self.end_byte)
                        .is_some_and(codeguard_adapters::cfquery_projection_span_valid)
            }
            _ => false,
        };
        rule_valid
            && self.basis == "codeguard_structure_rule"
            && self.rule_version == "1.0.0"
            && self.start_byte <= self.end_byte
            && position(source, self.start_byte) == Some((self.start_row, self.start_column_byte))
            && position(source, self.end_byte) == Some((self.end_row, self.end_column_byte))
    }
}
fn position(source: &[u8], offset: usize) -> Option<(usize, usize)> {
    let text = std::str::from_utf8(source).ok()?;
    if !text.is_char_boundary(offset) {
        return None;
    }
    let preceding = source.get(..offset)?;
    Some((
        preceding.iter().filter(|byte| **byte == b'\n').count(),
        preceding.rsplit(|byte| *byte == b'\n').next()?.len(),
    ))
}
