use super::ProjectionNativeRule;
use codeguard_core::NativeCheckerBinding;
use guard_protocol::{Digest, GitPath, Identifier};
/// 控制端批准的原生工具与义务映射；版本/制品声明仍需独立核验。无Java对应类型。
#[derive(Clone, Debug)]
pub struct ProjectionCheckerPin {
    pub verifier_id: Identifier,
    pub binding: NativeCheckerBinding,
    pub version: Identifier,
    pub artifact_digest: Digest,
    /// 事前批准的精确原生规则集合；空集合不能从反馈推断无违规。
    pub native_rules: Vec<ProjectionNativeRule>,
    /// 无路径依赖报告必须由宿主显式绑定源清单；不猜测Cargo.lock位置。
    pub feedback_target: Option<GitPath>,
}
