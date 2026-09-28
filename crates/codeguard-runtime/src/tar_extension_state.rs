//! 有界 GNU/PAX 解释层；不恢复权限、所有者、时间或扩展属性。
use std::collections::BTreeSet;
/// 下一普通条目的局部扩展状态；消费后清空，不得扩展到其它文件。
#[derive(Default)]
pub(crate) struct TarExtensionState {
    path: Option<String>,
    size: Option<u64>,
    gnu_seen: bool,
    pax_seen: bool,
}
impl TarExtensionState {
    /// 解析完整扩展数据；参数为原生类型及有界字节，返回固定原因。
    pub(crate) fn observe(&mut self, kind: u8, bytes: &[u8]) -> Result<(), &'static str> {
        if bytes.is_empty() || bytes.len() > 64 * 1024 {
            return Err("archive_extension_invalid");
        }
        match kind {
            b'L' => {
                if self.gnu_seen || bytes.len() > 1025 || bytes.last() != Some(&0) {
                    return Err("archive_extension_invalid");
                }
                let path = std::str::from_utf8(&bytes[..bytes.len() - 1])
                    .map_err(|_| "archive_extension_invalid")?;
                if path.is_empty() || path.chars().any(char::is_control) {
                    return Err("archive_extension_invalid");
                }
                self.merge_path(path)?;
                self.gnu_seen = true;
            }
            b'x' | b'g' => {
                let global = kind == b'g';
                if !global && self.pax_seen {
                    return Err("archive_extension_conflict");
                }
                let mut seen = BTreeSet::new();
                let mut offset = 0;
                while offset < bytes.len() {
                    // PAX 是带长度的记录，不按换行分割后忽略坏尾部；所有记录必须完整消费。
                    let remaining = &bytes[offset..];
                    let space = remaining
                        .iter()
                        .position(|b| *b == b' ')
                        .ok_or("archive_extension_invalid")?;
                    if space == 0
                        || space > 8
                        || !remaining[..space].iter().all(u8::is_ascii_digit)
                        || remaining[0] == b'0'
                    {
                        return Err("archive_extension_invalid");
                    }
                    let length = std::str::from_utf8(&remaining[..space])
                        .map_err(|_| "archive_extension_invalid")?
                        .parse::<usize>()
                        .map_err(|_| "archive_extension_invalid")?;
                    if length <= space + 2
                        || length > remaining.len()
                        || remaining[length - 1] != b'\n'
                    {
                        return Err("archive_extension_invalid");
                    }
                    let payload = &remaining[space + 1..length - 1];
                    let equals = payload
                        .iter()
                        .position(|b| *b == b'=')
                        .ok_or("archive_extension_invalid")?;
                    let key = std::str::from_utf8(&payload[..equals])
                        .map_err(|_| "archive_extension_invalid")?;
                    if key.is_empty()
                        || !key
                            .bytes()
                            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b'-'))
                        || !seen.insert(key)
                    {
                        return Err("archive_extension_invalid");
                    }
                    let value = &payload[equals + 1..];
                    if key == "linkpath"
                        || key.starts_with("GNU.sparse.")
                        || matches!(key, "SCHILY.filetype" | "SCHILY.realsize" | "hdrcharset")
                    {
                        return Err("archive_extension_unsupported");
                    }
                    match key {
                        "path" => {
                            if global {
                                return Err("archive_extension_unsupported");
                            }
                            let path = std::str::from_utf8(value)
                                .map_err(|_| "archive_extension_invalid")?;
                            if path.is_empty()
                                || path.len() > 1024
                                || path.chars().any(char::is_control)
                            {
                                return Err("archive_extension_invalid");
                            }
                            self.merge_path(path)?;
                        }
                        "size" => {
                            if global {
                                return Err("archive_extension_unsupported");
                            }
                            if value.is_empty() || !value.iter().all(u8::is_ascii_digit) {
                                return Err("archive_extension_invalid");
                            }
                            self.size = Some(
                                std::str::from_utf8(value)
                                    .map_err(|_| "archive_extension_invalid")?
                                    .parse()
                                    .map_err(|_| "archive_extension_invalid")?,
                            );
                        }
                        // 非路径/大小的描述字段仅消费，永远不保留到落盘权限或 xattr。
                        _ => {}
                    }
                    offset += length;
                }
                if !global {
                    self.pax_seen = true;
                }
            }
            _ => return Err("archive_extension_unsupported"),
        }
        Ok(())
    }
    /// 为当前文件取得生效路径和大小后清空状态；覆盖路径不要求截断的原头仍为 UTF-8。
    pub(crate) fn consume(
        &mut self,
        base_path: &[u8],
        header_size: u64,
    ) -> Result<(String, u64), &'static str> {
        let path = match self.path.take() {
            Some(path) => path,
            None => std::str::from_utf8(base_path)
                .map_err(|_| "archive_path_invalid")?
                .to_owned(),
        };
        let size = self.size.take().unwrap_or(header_size);
        self.gnu_seen = false;
        self.pax_seen = false;
        Ok((path, size))
    }
    /// 原头路径是否被完整扩展路径替代；返回值只控制有效路径解析。
    pub(crate) fn has_path_override(&self) -> bool {
        self.path.is_some()
    }
    /// 判断尾部是否留下没有对应文件的局部扩展。
    pub(crate) fn has_pending(&self) -> bool {
        self.gnu_seen || self.pax_seen
    }
    fn merge_path(&mut self, path: &str) -> Result<(), &'static str> {
        if self.path.as_deref().is_some_and(|old| old != path) {
            return Err("archive_extension_conflict");
        }
        self.path = Some(path.to_owned());
        Ok(())
    }
}
