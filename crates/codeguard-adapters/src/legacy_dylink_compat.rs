//! 将固定依赖包的旧版 dylink 元数据转换成当前 Rust WasmStore 所需格式。

use sha2::{Digest, Sha256};

/// 仅改写固定来源 WASM 的动态链接自定义段，并验证结果摘要。
/// 参数为语言名与原始字节；返回可由 Rust 加载的字节，不修改语法指令。
pub fn adapt_legacy_dylink(language: &str, source: &[u8]) -> Result<Vec<u8>, String> {
    let (source_sha, output_sha) = match language {
        "objc" => (
            "7c1b5bfdca7e64b6c63b6040bb7ba0afc347df116f9030ca32f8535d7377f6ff",
            "2606d4c5809fab61de44d328072d1371d53aa4aff5734436cb2a0ce8db7f2b0c",
        ),
        "solidity" => (
            "160745e470f234cae903a9ba445d19e758d0b02e1197401fc765976c6254d2b6",
            "ba02ba3c98c8ce976ed962d727ef48940b3a18dd2243830a8158b558de64b4f2",
        ),
        _ => return Err("未知的旧版 dylink grammar".into()),
    };
    if format!("{:x}", Sha256::digest(source)) != source_sha
        || !source.starts_with(b"\0asm\x01\0\0\0")
    {
        return Err("旧版 dylink grammar 来源字节不符".into());
    }
    let mut output = source[..8].to_vec();
    let mut offset = 8;
    let mut converted = false;
    while offset < source.len() {
        let start = offset;
        let id = source[offset];
        offset += 1;
        let (length, payload_start) = read_leb(source, offset)?;
        let end = payload_start
            .checked_add(length)
            .filter(|end| *end <= source.len())
            .ok_or("WASM section 越界")?;
        if id == 0 {
            let (name_len, name_start) = read_leb(source, payload_start)?;
            let name_end = name_start
                .checked_add(name_len)
                .filter(|end_name| *end_name <= end)
                .ok_or("WASM 自定义段名称越界")?;
            if &source[name_start..name_end] == b"dylink" {
                if converted {
                    return Err("重复 dylink 段".into());
                }
                let legacy = &source[name_end..end];
                let mut position = 0;
                for _ in 0..4 {
                    position = read_leb(legacy, position)?.1;
                }
                if legacy[position..].iter().any(|byte| *byte != 0) || legacy.len() != position + 1
                {
                    return Err("旧版 dylink 额外依赖布局不支持".into());
                }
                let mut body = vec![8];
                body.extend_from_slice(b"dylink.0");
                body.push(1);
                write_leb(position, &mut body);
                body.extend_from_slice(&legacy[..position]);
                output.push(0);
                write_leb(body.len(), &mut output);
                output.extend_from_slice(&body);
                converted = true;
                offset = end;
                continue;
            }
        }
        output.extend_from_slice(&source[start..end]);
        offset = end;
    }
    if !converted || format!("{:x}", Sha256::digest(&output)) != output_sha {
        return Err("旧版 dylink grammar 适配结果不符".into());
    }
    Ok(output)
}

fn read_leb(bytes: &[u8], mut offset: usize) -> Result<(usize, usize), String> {
    let mut value = 0usize;
    for shift in (0..35).step_by(7) {
        let byte = *bytes.get(offset).ok_or("WASM LEB 长度缺失")?;
        offset += 1;
        value |= usize::from(byte & 0x7f) << shift;
        if byte & 0x80 == 0 {
            return Ok((value, offset));
        }
    }
    Err("WASM LEB 长度无效".into())
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
            break;
        }
    }
}
