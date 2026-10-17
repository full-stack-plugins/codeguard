use super::ProjectionSourceProof;
use guard_protocol::{EvidenceRef, Identifier};
/// 本次Java原生输出与源码原件声明，只有Context绑定后可用于局部诊断匹配。无Java对应类型。
#[derive(Clone, Debug)]
pub(super) struct ProjectionJavaExecution {
    pub checker_id: Identifier,
    pub log: Vec<u8>,
    pub evidence: EvidenceRef,
    pub native_exit: i32,
    pub parser_version: Identifier,
    pub sources: Vec<ProjectionSourceProof>,
}
