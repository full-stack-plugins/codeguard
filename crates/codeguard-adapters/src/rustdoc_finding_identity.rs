//! 文档发现身份绑定原生源码范围；不靠诊断顺序给其它问题续用身份。

use crate::RustdocFinding;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

/// 以原生主定位对应的实际源码字节产生候选身份。
/// 参数为原生发现、该文件捕获字节及已核对的相对目标路径；返回未批准观察或范围错误。
pub fn rustdoc_finding_record(
    finding: &RustdocFinding,
    source: &[u8],
    target_relative: &str,
) -> Result<Value, &'static str> {
    let start = usize::try_from(finding.byte_start).map_err(|_| "native_finding_range_invalid")?;
    let end = usize::try_from(finding.byte_end).map_err(|_| "native_finding_range_invalid")?;
    let text = std::str::from_utf8(source).map_err(|_| "source_utf8_invalid")?;
    let range = text.get(start..end).ok_or("native_finding_range_invalid")?;
    let prefix = &text[..start];
    let expected_line = prefix.bytes().filter(|byte| *byte == b'\n').count() as u64 + 1;
    let expected_column = prefix.rsplit('\n').next().unwrap_or("").chars().count() as u64 + 1;
    if finding.line != expected_line || finding.column != expected_column {
        return Err("native_finding_location_mismatch");
    }
    let mut hash = Sha256::new();
    for part in [
        b"codeguard-rustdoc-native-span-v2".as_slice(),
        finding.rule_id.as_bytes(),
        finding.path.as_bytes(),
        target_relative.as_bytes(),
        range.as_bytes(),
    ] {
        hash.update((part.len() as u64).to_be_bytes());
        hash.update(part);
    }
    let fingerprint = format!("{:x}", hash.finalize());
    Ok(json!({
        "finding_id":format!("CG-{}",&fingerprint[..32]),"finding_fingerprint":fingerprint,
        "source_sha256":format!("{:x}",Sha256::digest(source)),
        "native_range_sha256":format!("{:x}",Sha256::digest(range.as_bytes())),
        "byte_start":finding.byte_start,"byte_end":finding.byte_end,
        "rule_id":finding.rule_id,"level":finding.level,"path":finding.path,
        "line":finding.line,"column":finding.column,"classification":"observed_unverified",
        "identity_status":"unique_candidate"
        ,"target_path":target_relative
    }))
}
