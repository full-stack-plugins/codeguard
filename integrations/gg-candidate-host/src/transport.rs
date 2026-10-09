//! Typed diagnostics before candidate binding. No envelope is constructible here.
use crate::PreparedCandidate;
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrebindingCode {
    BadParameters,
    UnknownRepository,
    MissingCandidateOrBase,
    UnfrozenScope,
    CandidateBindingInvalid,
}
impl PrebindingCode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::BadParameters => "bad_parameters",
            Self::UnknownRepository => "unknown_repository",
            Self::MissingCandidateOrBase => "missing_candidate_or_base",
            Self::UnfrozenScope => "unfrozen_scope",
            Self::CandidateBindingInvalid => "candidate_binding_invalid",
        }
    }
}
#[derive(Debug)]
pub struct PrebindingDiagnostic(PrebindingCode);
impl PrebindingDiagnostic {
    pub fn code(&self) -> PrebindingCode {
        self.0
    }
    /// Exact existing transport schema; never includes arbitrary input or an envelope.
    pub fn to_json(&self) -> Vec<u8> {
        serde_json::to_vec(&serde_json::json!({
            "apiVersion": "codeguard.transport/v1alpha1",
            "kind": "GuardTransportDiagnostic", "phase": "unbound",
            "code": self.0.as_str(), "exitCode": 4
        }))
        .expect("static diagnostic serializes")
    }
}
/// Explicit controller root and snapshot only. No repository discovery from cwd or labels.
/// Success still requires every check in the original `PreparedCandidate::from_bytes`.
pub fn prepare(
    bytes: &[u8],
    root: &Path,
    target: &str,
) -> Result<PreparedCandidate, PrebindingDiagnostic> {
    use PrebindingCode::*;
    if bytes.len() > 1_048_576
        || !root.is_absolute()
        || target.len() > 256
        || !target.ends_with(".py")
        || target.starts_with('/')
        || target.chars().any(|c| c.is_control() || c == '\\')
        || target
            .split('/')
            .any(|p| p.is_empty() || p == "." || p == ".." || p == ".git")
    {
        return Err(PrebindingDiagnostic(BadParameters));
    }
    // Reuse the native strict reader: raw bound above, default serde recursion limit,
    // recursive duplicate rejection, per-container item bounds. No input interpolation.
    let value = codeguard_adapters::parse_unique_json(bytes)
        .map_err(|_| PrebindingDiagnostic(BadParameters))?;
    let object = value
        .as_object()
        .ok_or(PrebindingDiagnostic(BadParameters))?;
    for field in ["candidate_oid", "base_oid"] {
        if object
            .get(field)
            .is_none_or(|v| v.is_null() || v.as_str() == Some(""))
        {
            return Err(PrebindingDiagnostic(MissingCandidateOrBase));
        }
    }
    for field in ["requirement_ids", "allowed_paths"] {
        if object
            .get(field)
            .is_none_or(|v| v.is_null() || v.as_array().is_some_and(Vec::is_empty))
        {
            return Err(PrebindingDiagnostic(UnfrozenScope));
        }
    }
    if object
        .get("policy_digest")
        .is_none_or(|v| v.is_null() || v.as_str() == Some(""))
    {
        return Err(PrebindingDiagnostic(UnfrozenScope));
    }
    // Classification is not validation; original bytes (not the Value) remain authoritative.
    drop(value);
    PreparedCandidate::from_bytes(bytes, root, target).map_err(|error| {
        PrebindingDiagnostic(match error {
            "invalid candidate input" | "invalid candidate snapshot" => BadParameters,
            "repository unavailable" => UnknownRepository,
            "target outside frozen scope" => UnfrozenScope,
            _ => CandidateBindingInvalid,
        })
    })
}
