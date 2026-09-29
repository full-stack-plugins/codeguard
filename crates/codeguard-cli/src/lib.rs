//! CLI 共享的 Rust 开发期验收与命令应用服务入口。

pub mod agents_block;
pub mod approval_snapshot;
mod bound_false_positive_disposition;
#[cfg(unix)]
pub mod doctor_command;
#[cfg(unix)]
mod doctor_scratch;
#[cfg(unix)]
mod eslint_config_map;
#[cfg(unix)]
mod eslint_directory;
mod eslint_discovery;
#[cfg(unix)]
mod eslint_effective_settings;
#[cfg(unix)]
mod eslint_lint_arguments;
#[cfg(unix)]
pub mod eslint_lint_command;
#[cfg(all(feature = "wasm-precheck", unix))]
mod eslint_native_first_candidate;
mod eslint_preparation;
pub mod eslint_probe;
pub mod eslint_probe_request;
pub mod eslint_probe_result;
#[cfg(unix)]
mod eslint_project_context;
mod eslint_syntax_evidence;
mod eslint_task_recheck;
#[cfg(unix)]
mod eslint_workbench;
#[cfg(unix)]
mod npm_audit_arguments;
#[cfg(unix)]
pub mod npm_audit_command;
mod npm_audit_discovery;
pub mod npm_audit_probe;
pub mod npm_audit_probe_request;
pub mod npm_audit_probe_result;
mod npm_historical_roots;
mod npm_input_state;
mod npm_observation_projection;
mod npm_recorded_roots;
mod npm_workbench;
#[cfg(unix)]
mod npm_workspace_scope;
mod python_dependency_discovery;
mod signed_disposition_binding;
pub mod tools_command;
pub use bound_false_positive_disposition::BoundFalsePositiveDisposition;
pub use signed_disposition_binding::bind_signed_false_positive_preview;
#[cfg(unix)]
pub use signed_disposition_binding::bind_signed_false_positive_replacement_preview;
#[cfg(unix)]
mod git_approval_baseline_context;
#[cfg(unix)]
mod git_approval_binding;
#[cfg(unix)]
pub use git_approval_baseline_context::GitApprovalBaselineContext;
#[cfg(unix)]
pub use git_approval_binding::verify_and_bind_replacement_chain_with_git;
mod approval_trust_key;
mod approval_verification_context;
mod signed_approval_envelope;
mod signed_approval_payload;
mod signed_approval_verifier;
mod signed_prior_approval_input;
mod verified_approval_snapshot;
pub use approval_trust_key::ApprovalTrustKey;
pub use approval_verification_context::ApprovalVerificationContext;
pub use signed_approval_verifier::verify_signed_approval;
pub use signed_prior_approval_input::SignedPriorApprovalInput;
pub use verified_approval_snapshot::VerifiedApprovalSnapshot;
pub mod cargo_audit_command;
pub mod cargo_build_repair_brief;
#[cfg(unix)]
pub mod check_budget;
#[cfg(unix)]
pub mod check_command;
pub mod check_plan;
pub mod check_request;
#[cfg(all(unix, feature = "wasm-precheck"))]
mod check_syntax_candidates;
mod checkstyle_preparation;
mod checkstyle_preparation_recheck;
pub mod checkstyle_probe;
pub mod checkstyle_probe_request;
pub mod checkstyle_probe_result;
mod checkstyle_task_recheck;
mod checkstyle_workbench;
#[cfg(unix)]
pub mod claude_hook_command;
pub mod config_command;
pub mod conversation_feedback;
pub mod corpus;
pub mod discovery;
pub mod false_positive_decision;
#[cfg(unix)]
pub mod git_index_safety;
#[cfg(unix)]
pub mod git_index_safety_command;
#[cfg(unix)]
pub mod go_lint_command;
#[cfg(feature = "wasm-precheck")]
pub mod grammar_probe_command;
#[cfg(feature = "wasm-precheck")]
pub mod grammar_route;
pub mod grammar_status_command;
#[cfg(unix)]
pub mod hook_execute_command;
pub mod hook_plan_command;
pub mod init_command;
mod java_checker_config_status;
pub mod java_checkstyle_command;
mod java_cve_attribution;
#[cfg(unix)]
mod java_cve_scan;
#[cfg(unix)]
mod java_dependency_scan;
#[cfg(unix)]
pub mod java_javadoc_command;
mod java_javadoc_scan;
#[cfg(unix)]
pub mod java_lint_dispatch;
#[cfg(unix)]
pub mod java_p3c_command;
mod java_p3c_scan;
#[cfg(feature = "wasm-precheck")]
mod java_syntax_precheck;
pub mod legacy_v1_protocol;
#[cfg(unix)]
mod maven_dependency_probe;
#[cfg(unix)]
mod maven_javadoc_probe;
#[cfg(unix)]
pub mod maven_probe;
mod native_tool_candidate;
mod native_tool_discovery;
pub mod next_command;
#[cfg(unix)]
mod owasp_maven_probe;
#[cfg(unix)]
pub mod partial_sarif_feedback;
pub mod plan_command;
#[cfg(unix)]
pub mod pmd6_probe;
#[cfg(unix)]
mod project_runtime_options;
#[cfg(unix)]
pub mod python_cve_command;
pub(crate) mod python_cve_task_recheck;
#[cfg(unix)]
pub mod python_lint_command;
#[cfg(unix)]
pub mod python_lint_scan;
#[cfg(unix)]
mod python_syntax_confirmation;
#[cfg(all(unix, feature = "wasm-precheck"))]
mod python_syntax_precheck;
pub mod quality_policy_candidate;
#[cfg(unix)]
mod report_export;
#[cfg(unix)]
pub mod ruff_probe;
mod ruff_verification_configuration;
pub mod rules_list_command;
pub mod run_report;
#[cfg(unix)]
pub mod rust_build_command;
pub(crate) mod rust_build_task_recheck;
#[cfg(unix)]
pub mod rust_comments_command;
pub(crate) mod rust_cve_task_recheck;
#[cfg(unix)]
pub mod rust_lint_scan;
pub mod rustdoc_repair_brief;
#[cfg(unix)]
pub(crate) mod rustdoc_task_recheck;
pub mod sarif_feedback;
#[cfg(feature = "wasm-precheck")]
mod syntax_worker_candidate_observation;
#[cfg(feature = "wasm-precheck")]
pub mod syntax_worker_command;
#[cfg(feature = "wasm-precheck")]
mod syntax_worker_envelope;
#[cfg(feature = "wasm-precheck")]
mod syntax_worker_recovery;
#[cfg(feature = "wasm-precheck")]
pub mod syntax_worker_runner;
#[cfg(unix)]
pub mod task_attempt_command;
#[cfg(unix)]
pub mod task_lease_command;
pub mod task_verify_command;
pub mod tool_identity;
pub mod tool_lock;
#[cfg(feature = "wasm-precheck")]
mod typescript_syntax_precheck;
pub mod whitelist_command;
mod whitelist_correction_command;
mod whitelist_correction_projection;
pub mod work_sync;
pub mod workspace_refresh;
pub mod workspace_view_command;
#[cfg(all(feature = "wasm-precheck", unix))]
pub mod zig_lint_command;

mod cargo_module_graph;
mod module_graph;

mod language_target_profile;

mod init_profile_feedback;

pub mod distribution_artifact;
pub mod distribution_bundle;
pub mod distribution_layout;
pub mod distribution_raw;
mod raw_distribution;
pub use raw_distribution::RawDistribution;
mod distribution_layout_result;
pub use distribution_layout_result::DistributionLayout;
pub mod distribution_manifest;

mod distribution_install_preview;
mod distribution_layout_preview;

mod bound_distribution_source;
pub mod distribution_source;
mod distribution_trust_key;
mod distribution_verification_context;
mod signed_distribution_envelope;
mod signed_distribution_payload;

#[cfg(any(target_os = "macos", target_os = "linux"))]
pub mod distribution_publication;
#[cfg(any(target_os = "macos", target_os = "linux"))]
mod distribution_publication_request;
#[cfg(any(target_os = "macos", target_os = "linux"))]
mod published_distribution_artifact;

#[cfg(any(target_os = "macos", target_os = "linux"))]
pub mod distribution_download;
#[cfg(any(target_os = "macos", target_os = "linux"))]
mod signed_distribution_download_request;

mod npm_task_recheck;

mod npm_check_scan;
