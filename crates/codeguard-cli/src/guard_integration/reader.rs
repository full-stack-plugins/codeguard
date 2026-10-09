use super::profile::InvocationDescriptor;
use crate::run_report::{RunReport, parse_run_report};
use sha2::{Digest, Sha256};

/// Native evidence remains domain data, never an engine decision or proof of authority.
pub struct NativeEvidence {
    raw: Vec<u8>,
    digest: String,
    report: RunReport,
}
impl NativeEvidence {
    pub fn raw_bytes(&self) -> &[u8] {
        &self.raw
    }
    pub fn raw_sha256(&self) -> &str {
        &self.digest
    }
    pub fn report(&self) -> &RunReport {
        &self.report
    }
}
pub fn read_native(
    bytes: &[u8],
    invocation: &InvocationDescriptor,
) -> Result<NativeEvidence, &'static str> {
    invocation.validate()?;
    if bytes.len() > 1024 * 1024 {
        return Err("native report exceeds limit");
    }
    let report = parse_run_report(bytes).map_err(|_| "invalid native report")?;
    let document = report.document();
    if document["schema_version"] != invocation.native_schema
        || document["operation"] != invocation.command
        || report.run_id != invocation.run_id
        || report.request_id != invocation.request_id
        || report.exit_code != invocation.process_exit
        || document["request"]["options"]["format"] != "json"
    {
        return Err("native report conflicts with invocation");
    }
    Ok(NativeEvidence {
        raw: bytes.to_vec(),
        digest: format!("{:x}", Sha256::digest(bytes)),
        report,
    })
}
