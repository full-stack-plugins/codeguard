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
mod hook_event;
mod hook_scope_resolution_reason;
mod hook_trigger_action;
mod hook_trigger_input;
mod hook_trigger_plan;
mod hook_trigger_planner;
mod hook_write_outcome;
mod native_checker_binding;
mod native_fallback_state;
mod obligation_result;
mod observation_port;
mod repository_path_safety;
mod syntax_file_observation;
mod syntax_file_state;
mod syntax_incomplete_reason;
mod syntax_precheck;
mod syntax_precheck_outcome;
mod syntax_precheck_status;
mod syntax_recovery_anchor;
mod syntax_setup_action;
mod syntax_setup_guidance;
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
pub use hook_event::HookEvent;
pub use hook_scope_resolution_reason::HookScopeResolutionReason;
pub use hook_trigger_action::HookTriggerAction;
pub use hook_trigger_input::HookTriggerInput;
pub use hook_trigger_plan::HookTriggerPlan;
pub use hook_trigger_planner::plan_hook_trigger;
pub use hook_write_outcome::HookWriteOutcome;
pub use native_checker_binding::NativeCheckerBinding;
pub use native_fallback_state::NativeFallbackState;
pub use obligation_result::ObligationResult;
pub use observation_port::{ObservationPort, ObservedPathKind};
pub use repository_path_safety::{RepositoryPathViolation, check_repository_paths};
pub use syntax_file_observation::SyntaxFileObservation;
pub use syntax_file_state::SyntaxFileState;
pub use syntax_incomplete_reason::{
    INCOMPLETE_REASON, REASON_PARSER_ERROR_UNLOCATED, REASON_SCAN_TRUNCATED,
    is_incomplete_syntax_reason,
};
pub use syntax_precheck::assess_syntax_precheck;
pub use syntax_precheck_outcome::SyntaxPrecheckOutcome;
pub use syntax_precheck_status::SyntaxPrecheckStatus;
pub use syntax_recovery_anchor::SyntaxRecoveryAnchor;
pub use syntax_setup_action::SyntaxSetupAction;
pub use syntax_setup_guidance::select_syntax_setup_action;
pub use task_graph::{TaskGraph, TaskGraphError};
pub use task_node::TaskNode;
pub use verdict::Verdict;

mod readiness_state;
mod repository_content_safety;
pub use readiness_state::ReadinessState;
pub use repository_content_safety::has_unencrypted_openssh_ed25519_private_key;

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

mod resolution_cause;
mod resolution_evidence;
mod resolution_outcome;
mod task_identity;
mod task_lifecycle_event;
mod task_lifecycle_kind;
mod task_lifecycle_state;
mod task_lifecycle_view;
mod task_resolution;
pub use resolution_cause::ResolutionCause;
pub use resolution_evidence::ResolutionEvidence;
pub use resolution_outcome::ResolutionOutcome;
pub use task_identity::TaskIdentity;
pub use task_lifecycle_event::TaskLifecycleEvent;
pub use task_lifecycle_kind::TaskLifecycleKind;
pub use task_lifecycle_state::TaskLifecycleState;
pub use task_lifecycle_view::TaskLifecycleView;
pub use task_resolution::{evaluate_resolution, reduce_task_lifecycle};

mod task_resolution_policy;
pub use task_resolution_policy::TaskResolutionPolicy;
mod task_lifecycle_record;
pub use task_lifecycle_record::TaskLifecycleRecord;

pub mod finding_identity;

pub mod run_config;

pub mod typed_module_graph;
