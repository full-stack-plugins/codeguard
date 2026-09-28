use codeguard_core::SyntaxRecoveryAnchor;

use crate::SourceMappedRecovery;

/// 一份已验证 UTF-8 源码的行起点索引，用于多条恢复节点的有界位置核对。
pub struct SourceMap<'a> {
    text: &'a str,
    line_starts: Vec<usize>,
}

impl<'a> SourceMap<'a> {
    /// 验证源码编码与大小，并构建行起点索引；参数是原始源码字节。
    pub fn new(source: &'a [u8]) -> Result<Self, String> {
        if source.len() > 1024 * 1024 {
            return Err("语法初检源码超过大小上限".into());
        }
        let text = std::str::from_utf8(source).map_err(|_| "源码不是 UTF-8，位置映射不完整")?;
        let mut line_starts = vec![0];
        line_starts.extend(
            source
                .iter()
                .enumerate()
                .filter_map(|(index, byte)| (*byte == b'\n').then_some(index + 1)),
        );
        Ok(Self { text, line_starts })
    }

    /// 核对语法树原始字节/行列锚点并转为一起始 Unicode 标量列。
    /// 不猜测插入位置或缺失 token；不一致时返回不完整错误。
    pub fn map(&self, anchor: &SyntaxRecoveryAnchor) -> Result<SourceMappedRecovery, String> {
        if !matches!(anchor.kind, "ERROR" | "MISSING")
            || anchor.group_id == 0
            || anchor.syntax_kind.is_empty()
            || anchor.syntax_kind.len() > 128
            || anchor.start_byte > anchor.end_byte
            || anchor.end_byte > self.text.len()
        {
            return Err("语法恢复锚点无效".into());
        }
        let (start_row, start_column_byte, start_column_scalar) = self.point(anchor.start_byte)?;
        let (end_row, end_column_byte, end_column_scalar) = self.point(anchor.end_byte)?;
        if (start_row, start_column_byte) != (anchor.start_row, anchor.start_column_byte)
            || (end_row, end_column_byte) != (anchor.end_row, anchor.end_column_byte)
        {
            return Err("语法恢复锚点与源码位置不符".into());
        }
        Ok(SourceMappedRecovery {
            kind: anchor.kind,
            group_id: anchor.group_id,
            syntax_kind: anchor.syntax_kind.clone(),
            start_byte: anchor.start_byte,
            end_byte: anchor.end_byte,
            start_line: start_row + 1,
            start_column_scalar,
            end_line: end_row + 1,
            end_column_scalar,
        })
    }

    fn point(&self, offset: usize) -> Result<(usize, usize, usize), String> {
        if !self.text.is_char_boundary(offset) {
            return Err("语法恢复锚点落在 UTF-8 字符内部".into());
        }
        let row = self.line_starts.partition_point(|&start| start <= offset) - 1;
        let line_start = self.line_starts[row];
        let column_byte = offset - line_start;
        let column_scalar = self.text[line_start..offset].chars().count() + 1;
        Ok((row, column_byte, column_scalar))
    }
}

/// 单条语法恢复锚点的便捷映射；批量映射请复用 SourceMap。
pub fn map_syntax_recovery(
    source: &[u8],
    anchor: &SyntaxRecoveryAnchor,
) -> Result<SourceMappedRecovery, String> {
    SourceMap::new(source)?.map(anchor)
}
