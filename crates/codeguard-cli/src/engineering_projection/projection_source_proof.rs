use super::ProjectionRun;
use guard_protocol::{
    Classification, Digest, EvidenceRef, ExactTarget, Identifier, VerificationPlan,
};
use sha2::{Digest as Hasher, Sha256};
/// 宿主独立捕获的本次精确源码字节与私有原件引用；不接受报告JSON自证，不授予可信身份。无Java对应类型。
#[derive(Clone, Debug)]
pub struct ProjectionSourceProof {
    pub(super) target: ExactTarget,
    pub(super) native_path: String,
    pub(super) bytes: Vec<u8>,
    pub(super) evidence: EvidenceRef,
    pub(super) run_id: Identifier,
    pub(super) subject_digest: Digest,
}
impl ProjectionSourceProof {
    /// 校验宿主捕获字节的哈希/长度/私有引用及冻结目标，参数native_path为本次原生临时路径的显式映射。
    /// 此函数不读盘、不执行采集；控制端仍负责K04/K06捕获真实性。
    pub fn new(
        plan: &VerificationPlan,
        run: &ProjectionRun,
        target: &ExactTarget,
        native_path: &str,
        bytes: &[u8],
        evidence: &EvidenceRef,
    ) -> Result<Self, String> {
        if !plan.targets.contains(target) || native_path.is_empty() {
            return Err("source proof outside frozen targets/native path missing".into());
        }
        validate_evidence(bytes, evidence)?;
        if evidence.digest != target.input_digest {
            return Err("captured source differs from frozen target".into());
        }
        Ok(Self {
            target: target.clone(),
            native_path: native_path.into(),
            bytes: bytes.to_vec(),
            evidence: evidence.clone(),
            run_id: run.run_id.clone(),
            subject_digest: plan.subject_digest.clone(),
        })
    }
}
pub(super) fn validate_evidence(bytes: &[u8], evidence: &EvidenceRef) -> Result<(), String> {
    if bytes.len() > 16 * 1024 * 1024
        || evidence.classification != Classification::Private
        || evidence.size != bytes.len() as u64
        || evidence.digest.as_str() != format!("sha256:{:x}", Sha256::digest(bytes))
    {
        return Err("private captured evidence digest/size/budget mismatch".into());
    }
    Ok(())
}
