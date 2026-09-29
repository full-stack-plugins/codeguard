//! 拟入库内容的低误报结构检查；只识别完整的未加密 OpenSSH Ed25519 私钥。

use base64::Engine;
use base64::engine::general_purpose::STANDARD;

const BEGIN: &[u8] = b"-----BEGIN OPENSSH PRIVATE KEY-----";
const END: &[u8] = b"-----END OPENSSH PRIVATE KEY-----";
const MAX_ENCODED_BYTES: usize = 16 * 1024;

/// 在已核对身份的 Git blob 字节中识别完整的未加密 OpenSSH Ed25519 私钥。
/// 参数为原始 blob 字节，返回值只表示该精确格式命中；其他密钥格式仍须独立检查。
#[must_use]
pub fn has_unencrypted_openssh_ed25519_private_key(content: &[u8]) -> bool {
    let mut encoded: Option<Vec<u8>> = None;
    for line in content.split(|byte| *byte == b'\n') {
        let line = line.strip_suffix(b"\r").unwrap_or(line);
        if line == BEGIN {
            encoded = Some(Vec::new());
        } else if line == END {
            if let Some(encoded) = encoded.take() {
                if let Ok(decoded) = STANDARD.decode(&encoded) {
                    if valid_ed25519_envelope(&decoded) {
                        return true;
                    }
                }
            }
        } else if let Some(buffer) = encoded.as_mut() {
            if line.is_empty()
                || buffer.len().saturating_add(line.len()) > MAX_ENCODED_BYTES
                || !line
                    .iter()
                    .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'+' | b'/' | b'='))
            {
                encoded = None;
            } else {
                buffer.extend_from_slice(line);
            }
        }
    }
    false
}

fn valid_ed25519_envelope(bytes: &[u8]) -> bool {
    let Some(mut cursor) = bytes.strip_prefix(b"openssh-key-v1\0") else {
        return false;
    };
    let Some(cipher) = take_string(&mut cursor) else {
        return false;
    };
    let Some(kdf) = take_string(&mut cursor) else {
        return false;
    };
    let Some(kdf_options) = take_string(&mut cursor) else {
        return false;
    };
    if cipher != b"none" || kdf != b"none" || !kdf_options.is_empty() {
        return false;
    }
    let Some(key_count) = take_u32(&mut cursor) else {
        return false;
    };
    if key_count != 1 {
        return false;
    }
    let Some(public_blob) = take_string(&mut cursor) else {
        return false;
    };
    let Some(private_blob) = take_string(&mut cursor) else {
        return false;
    };
    if !cursor.is_empty() || private_blob.len() % 8 != 0 {
        return false;
    }
    let mut public = public_blob;
    if take_string(&mut public) != Some(b"ssh-ed25519".as_slice()) {
        return false;
    }
    let Some(public_key) = take_string(&mut public) else {
        return false;
    };
    if public_key.len() != 32 || !public.is_empty() {
        return false;
    }
    let mut private = private_blob;
    let Some(first_check) = take_u32(&mut private) else {
        return false;
    };
    if take_u32(&mut private) != Some(first_check)
        || take_string(&mut private) != Some(b"ssh-ed25519".as_slice())
        || take_string(&mut private) != Some(public_key)
    {
        return false;
    }
    let Some(key_material) = take_string(&mut private) else {
        return false;
    };
    if key_material.len() != 64 || key_material[32..] != *public_key {
        return false;
    }
    if take_string(&mut private).is_none() || private.len() > 8 {
        return false;
    }
    private
        .iter()
        .enumerate()
        .all(|(index, byte)| *byte == (index + 1) as u8)
}

fn take_u32(input: &mut &[u8]) -> Option<u32> {
    let (head, tail) = input.split_at_checked(4)?;
    *input = tail;
    Some(u32::from_be_bytes(head.try_into().ok()?))
}

fn take_string<'a>(input: &mut &'a [u8]) -> Option<&'a [u8]> {
    let size = usize::try_from(take_u32(input)?).ok()?;
    let (head, tail) = input.split_at_checked(size)?;
    *input = tail;
    Some(head)
}
