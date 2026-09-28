//! 原生 ESLint 发现到脱敏任务输入的稳定投影；不赋予策略或报告来源权威。
use codeguard_adapters::EslintDiagnostic;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, path::Path};

/// 将完整单文件原生规则发现绑定到相对路径、源码锚点与稳定序号。
/// 参数为工作区相对路径、冻结原生绝对路径、同轮源码及诊断集合。
/// 任一定位无效返回空，拒绝部分任务输入；返回值仍不是批准或关闭证据。
#[must_use]
pub fn project_eslint_findings(
    relative: &str,
    absolute: &str,
    source: &[u8],
    findings: &[EslintDiagnostic],
) -> Option<Vec<Value>> {
    if relative.is_empty()
        || relative.len() > 4096
        || Path::new(relative).is_absolute()
        || relative.contains('\\')
        || relative.chars().any(char::is_control)
        || relative
            .split('/')
            .any(|part| matches!(part, "" | "." | ".."))
        || !Path::new(absolute).is_absolute()
        || absolute.chars().any(char::is_control)
        || source.len() > 16 * 1024 * 1024
        || findings.len() > 100_000
    {
        return None;
    }
    let text = std::str::from_utf8(source).ok()?;
    // CRLF 是一个终止符；JavaScript 同时支持单独 CR 和 Unicode 行分隔符。
    let normalized = text.replace("\r\n", "\n");
    let lines: Vec<_> = normalized
        .split(['\n', '\r', '\u{2028}', '\u{2029}'])
        .collect();
    let mut ordered = findings.to_vec();
    ordered.sort_by(|left, right| {
        (
            left.line,
            left.column,
            &left.rule_id,
            left.severity,
            &left.message,
        )
            .cmp(&(
                right.line,
                right.column,
                &right.rule_id,
                right.severity,
                &right.message,
            ))
    });
    ordered.dedup_by(|right, left| {
        (right.line, right.column, &right.rule_id, right.severity)
            == (left.line, left.column, &left.rule_id, left.severity)
    });
    let source_sha = format!("{:x}", Sha256::digest(source));
    let mut occurrences = BTreeMap::<String, u64>::new();
    let mut records = Vec::new();
    for finding in ordered {
        if finding.path != absolute
            || !matches!(finding.severity, 1 | 2)
            || finding.rule_id.is_empty()
            || finding.rule_id.len() > 512
            || finding.rule_id.chars().any(char::is_control)
            || finding.message.is_empty()
            || finding.message.len() > 4096
        {
            return None;
        }
        let line = lines.get(usize::try_from(finding.line.checked_sub(1)?).ok()?)?;
        if finding.column == 0
            || usize::try_from(finding.column).ok()? > line.encode_utf16().count() + 1
        {
            return None;
        }
        let anchor = line.trim();
        if anchor.is_empty() {
            return None;
        }
        let mut hash = Sha256::new();
        for part in [
            b"codeguard-eslint-finding-v1".as_slice(),
            relative.as_bytes(),
            finding.rule_id.as_bytes(),
            anchor.as_bytes(),
        ] {
            hash.update((part.len() as u64).to_be_bytes());
            hash.update(part);
        }
        let base = format!("{:x}", hash.finalize());
        let ordinal = occurrences.entry(base.clone()).or_default();
        let mut hash = Sha256::new();
        hash.update(base.as_bytes());
        hash.update(ordinal.to_be_bytes());
        *ordinal += 1;
        let fingerprint = format!("{:x}", hash.finalize());
        records.push(json!({"finding_id":format!("CG-{}",&fingerprint[..32]),"finding_fingerprint":fingerprint,"source_sha256":source_sha,"path":relative,"rule_id":finding.rule_id,"line":finding.line,"native":{"rule_id":finding.rule_id,"line":finding.line,"column":finding.column,"severity":finding.severity,"message_sha256":format!("{:x}",Sha256::digest(finding.message.as_bytes()))}}));
    }
    Some(records)
}

/// 复核脱敏任务输入的稳定投影；消息摘要只验证形状，不自证原生来源。
pub(crate) fn validate_records(
    relative: &str,
    absolute: &str,
    source: &[u8],
    records: &[Value],
) -> bool {
    let mut diagnostics = Vec::new();
    for record in records {
        let native = &record["native"];
        if !exact(
            record,
            &[
                "finding_id",
                "finding_fingerprint",
                "source_sha256",
                "path",
                "rule_id",
                "line",
                "native",
            ],
        ) || !exact(
            native,
            &["rule_id", "line", "column", "severity", "message_sha256"],
        ) || !native["message_sha256"].as_str().is_some_and(|sha| {
            sha.len() == 64
                && sha
                    .bytes()
                    .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
        }) {
            return false;
        }
        let Some(rule) = native["rule_id"].as_str() else {
            return false;
        };
        let Some(line) = native["line"]
            .as_u64()
            .and_then(|value| u32::try_from(value).ok())
        else {
            return false;
        };
        let Some(column) = native["column"]
            .as_u64()
            .and_then(|value| u32::try_from(value).ok())
        else {
            return false;
        };
        let Some(severity) = native["severity"]
            .as_u64()
            .and_then(|value| u8::try_from(value).ok())
        else {
            return false;
        };
        diagnostics.push(EslintDiagnostic {
            path: absolute.into(),
            rule_id: rule.into(),
            line,
            column,
            severity,
            message: "unverified native message".into(),
        });
    }
    let Some(mut expected) = project_eslint_findings(relative, absolute, source, &diagnostics)
    else {
        return false;
    };
    if expected.len() != records.len() {
        return false;
    }
    for (wanted, actual) in expected.iter_mut().zip(records) {
        wanted["native"]["message_sha256"] = actual["native"]["message_sha256"].clone();
    }
    expected == records
}
fn exact(value: &Value, keys: &[&str]) -> bool {
    value
        .as_object()
        .is_some_and(|map| map.len() == keys.len() && keys.iter().all(|key| map.contains_key(*key)))
}
