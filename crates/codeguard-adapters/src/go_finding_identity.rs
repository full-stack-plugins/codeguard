//! Go 原生诊断的局部稳定身份；行号用于定位，源码字节摘要用于本轮归属复核。

use crate::GoVetFinding;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

/// 绑定诊断到源码行并生成脱敏记录。
///
/// 参数为模块相对根、原生诊断、本轮源码字节与同锚点序号；位置无效时返回空。
/// 稳定身份不含绝对行号或整文件摘要，因此前置空行不创建新问题。
#[must_use]
pub fn go_finding_record(
    module_root: &str,
    finding: &GoVetFinding,
    source: &[u8],
    occurrence: u64,
) -> Option<Value> {
    let line = source
        .split(|byte| *byte == b'\n')
        .nth(finding.line.checked_sub(1)? as usize)?;
    if finding.column == 0 || finding.column as usize > line.len() + 1 {
        return None;
    }
    let anchor = line.trim_ascii();
    if anchor.is_empty() {
        return None;
    }
    let mut hash = Sha256::new();
    for part in [
        b"codeguard-go-vet-finding-v1".as_slice(),
        module_root.as_bytes(),
        finding.package.as_bytes(),
        finding.path.as_bytes(),
        finding.native_rule_id.as_bytes(),
        anchor,
        finding.message.as_bytes(),
    ] {
        hash.update((part.len() as u64).to_be_bytes());
        hash.update(part);
    }
    hash.update(occurrence.to_be_bytes());
    let fingerprint = format!("{:x}", hash.finalize());
    Some(json!({
        "finding_id":format!("CG-{}", &fingerprint[..32]),
        "finding_fingerprint":fingerprint,
        "source_sha256":format!("{:x}", Sha256::digest(source)),
        "module_root":module_root,
        "rule_id":finding.native_rule_id,
        "path":finding.path,
        "line":finding.line,
        "column":finding.column,
        "native_message_sha256":format!("{:x}", Sha256::digest(finding.message.as_bytes())),
        "message_status":"native_text_private_untrusted"
    }))
}
