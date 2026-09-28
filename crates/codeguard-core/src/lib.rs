//! Codeguard 领域契约与纯判定，不依赖进程、文件系统或宿主 SDK。

mod aggregate;
mod allowlist_disposition;
mod approval_scope;
mod capability_dimensions;
mod check_session;
mod completion;
mod delivery_gate;
mod evaluation;
mod false_positive_allowlist;
mod finding;
mod native_checker_binding;
mod obligation_result;
mod observation_port;
mod repository_path_safety;
mod task_graph;
mod task_node;
mod verdict;

pub use aggregate::{AggregatedResult, aggregate};
pub use allowlist_disposition::AllowlistDisposition;
pub use approval_scope::ApprovalScope;
pub use capability_dimensions::{
    CANDIDATE_PLATFORMS, CHECK_CATEGORIES, CHECK_KINDS, check_kind_belongs_to_category,
};
pub use check_session::{CheckSessionInput, CheckSessionOutcome, conclude_check};
pub use completion::Completion;
pub use delivery_gate::{
    DeliveryDecision, DeliveryGate, DeliveryInput, ObligationEvidence, ObligationSpec,
    evaluate_delivery,
};
pub use evaluation::{
    EvaluationCase, EvaluationCounts, EvaluationOutcome, EvaluationStratum, EvaluationThresholds,
    OracleDecision, PrecisionEstimate, QualityEvaluation, evaluate_quality,
};
pub use false_positive_allowlist::{
    AllowlistTarget, FalsePositiveIdentity, IdentityMismatch, match_false_positive_identity,
    valid_false_positive_identity,
};
pub use finding::{Finding, FindingLocation, GateImpact};
pub use native_checker_binding::NativeCheckerBinding;
pub use obligation_result::ObligationResult;
pub use observation_port::{ObservationPort, ObservedPathKind};
pub use repository_path_safety::{RepositoryPathViolation, check_repository_paths};
pub use task_graph::{TaskGraph, TaskGraphError};
pub use task_node::TaskNode;
pub use verdict::Verdict;

mod readiness_state;
pub use readiness_state::ReadinessState;

mod preparation_evidence_state;
pub use preparation_evidence_state::PreparationEvidenceState;

mod prerequisite_requirement;
pub use prerequisite_requirement::PrerequisiteRequirement;

mod prerequisite_observation;
pub use prerequisite_observation::PrerequisiteObservation;

mod readiness_input;
pub use readiness_input::ReadinessInput;

mod readiness_outcome;
pub use readiness_outcome::ReadinessOutcome;

mod readiness;
pub use readiness::evaluate_readiness;

mod preparation_action;
pub use preparation_action::PreparationAction;
mod preparation_task;
pub use preparation_task::PreparationTask;
mod preparation_plan;
pub use preparation_plan::PreparationPlan;
mod preparation_planner;
pub use preparation_planner::plan_preparation;
