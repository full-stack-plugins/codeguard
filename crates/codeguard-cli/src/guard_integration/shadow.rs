//! Read-only comparison of original native bytes and an already sealed projection.
//!
//! Native-only summaries are byte identities, including for unsupported native formats:
//! they make no completed-run, qualification, or engine-decision claim. Projection
//! constructors remain the sole profile validation boundary. No envelope or I/O occurs.
use super::projection::Projection;
use guardengine::{GuardContract, GuardReport};
use sha2::{Digest, Sha256};

pub struct ShadowComparison<'a> {
    native_sha256: String,
    engine_report: Option<&'a GuardReport>,
}
impl ShadowComparison<'_> {
    pub fn native_sha256(&self) -> &str {
        &self.native_sha256
    }
    /// An engine-required consumer must reject a native-only byte summary.
    pub fn engine_report(&self) -> Result<&GuardReport, &'static str> {
        self.engine_report
            .ok_or("engine-backed projection required")
    }
}

pub fn compare<'a>(
    native: &[u8],
    projection: Option<&'a Projection>,
) -> Result<ShadowComparison<'a>, &'static str> {
    if native.len() > 1_048_576 {
        return Err("native shadow input exceeds byte budget");
    }
    let engine_report = if let Some(projection) = projection {
        if native != projection.domain_bytes() {
            return Err("shadow native bytes differ from sealed projection");
        }
        let contract: GuardContract = serde_json::from_slice(&projection.contract_bytes)
            .map_err(|_| "invalid sealed contract")?;
        let recomputed = guardengine::integration::evaluate_bounded(&contract, projection.facts())
            .map_err(|_| "shadow engine recomputation failed")?;
        if serde_json::to_value(&recomputed).map_err(|_| "cannot compare report")?
            != serde_json::to_value(projection.report()).map_err(|_| "cannot compare report")?
        {
            return Err("shadow engine report mismatch");
        }
        Some(projection.report())
    } else {
        None
    };
    Ok(ShadowComparison {
        native_sha256: format!("{:x}", Sha256::digest(native)),
        engine_report,
    })
}
