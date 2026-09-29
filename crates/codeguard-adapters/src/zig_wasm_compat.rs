//! 固定 CodeGraph Zig grammar 的可复现 Rust WasmStore 导入适配。

use sha2::{Digest, Sha256};

const SOURCE_SHA256: &str = "95d8eef504bde9cca06b7950d6a8ae177ce318d16080deac96f653b29c629f98";
const ADAPTED_SHA256: &str = "e8a3aa89cc07b59188122e6c1e0a812ca58cf85c9e191dcf2663a30b6b3b99e3";
const OLD_IMPORT: &[u8] = b"\x10__main_argc_argv";
const NEW_IMPORT: &[u8] = b"\x08args_get";

/// 将固定 Zig 源 WASM 的不兼容导入替换为同签名的运行时内置导入。
/// 参数为固定 CodeGraph 原始字节；返回适配字节。只有原始及结果散列都准确匹配才成功。
/// 适配仅触及 import section；不会重写 grammar、源码或解析树。
pub fn adapt_zig_wasm(source: &[u8]) -> Result<Vec<u8>, String> {
    if source.len() != 705336 || format!("{:x}", Sha256::digest(source)) != SOURCE_SHA256 {
        return Err("Zig grammar 来源字节不符".into());
    }
    let mut offset = 8;
    while offset < source.len() {
        let section_start = offset;
        let section_id = *source.get(offset).ok_or("WASM section 缺失")?;
        offset += 1;
        let (section_len, payload_start) = read_leb(source, offset)?;
        let payload_end = payload_start
            .checked_add(section_len)
            .filter(|end| *end <= source.len())
            .ok_or("WASM section 越界")?;
        if section_id == 2 {
            let payload = &source[payload_start..payload_end];
            let matches = payload
                .windows(OLD_IMPORT.len())
                .enumerate()
                .filter_map(|(index, bytes)| (bytes == OLD_IMPORT).then_some(index))
                .collect::<Vec<_>>();
            let [position] = matches.as_slice() else {
                return Err("Zig grammar 导入布局不符".into());
            };
            let mut adapted_payload = Vec::with_capacity(payload.len() - 8);
            adapted_payload.extend_from_slice(&payload[..*position]);
            adapted_payload.extend_from_slice(NEW_IMPORT);
            adapted_payload.extend_from_slice(&payload[position + OLD_IMPORT.len()..]);
            let mut adapted = Vec::with_capacity(source.len() - 8);
            adapted.extend_from_slice(&source[..section_start + 1]);
            write_leb(adapted_payload.len(), &mut adapted);
            adapted.extend_from_slice(&adapted_payload);
            adapted.extend_from_slice(&source[payload_end..]);
            if format!("{:x}", Sha256::digest(&adapted)) != ADAPTED_SHA256 {
                return Err("Zig grammar 适配结果不符".into());
            }
            return Ok(adapted);
        }
        offset = payload_end;
    }
    Err("Zig grammar 无导入区段".into())
}

fn read_leb(source: &[u8], mut offset: usize) -> Result<(usize, usize), String> {
    let mut result = 0usize;
    for shift in (0..35).step_by(7) {
        let byte = *source.get(offset).ok_or("WASM section 长度缺失")?;
        offset += 1;
        result |= usize::from(byte & 0x7f) << shift;
        if byte & 0x80 == 0 {
            return Ok((result, offset));
        }
    }
    Err("WASM section 长度无效".into())
}

fn write_leb(mut value: usize, output: &mut Vec<u8>) {
    loop {
        let mut byte = (value & 0x7f) as u8;
        value >>= 7;
        if value != 0 {
            byte |= 0x80;
        }
        output.push(byte);
        if value == 0 {
            return;
        }
    }
}
