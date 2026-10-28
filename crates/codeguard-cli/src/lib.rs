//! CLI 共享的 Rust 开发期验收与命令应用服务入口。

pub mod agents_block;
pub mod approval_snapshot;
mod bound_false_positive_disposition;
#[cfg(any(unix, windows))]
mod cargo_tool_selection;
#[cfg(any(unix, windows))]
mod check_erlang_scan;
mod command_catalogue;
mod command_descriptor;
mod command_examples;
#[cfg(any(unix, windows))]
pub mod doctor_command;
#[cfg(any(unix, windows))]
mod doctor_scratch;
#[cfg(any(unix, windows))]
mod erlang_lint_arguments;
#[cfg(any(unix, windows))]
pub mod erlang_lint_command;
#[cfg(any(unix, windows))]
mod erlang_syntax_probe;
#[cfg(any(unix, windows))]
mod erlang_tool_selection;
#[cfg(any(unix, windows))]
mod eslint_config_map;
#[cfg(any(unix, windows))]
mod eslint_directory;
mod eslint_discovery;
#[cfg(any(unix, windows))]
mod eslint_effective_settings;
#[cfg(any(unix, windows))]
mod eslint_lint_arguments;
#[cfg(any(unix, windows))]
pub mod eslint_lint_command;
#[cfg(any(unix, windows))]
mod eslint_native_first_candidate;
mod eslint_preparation;
pub mod eslint_probe;
pub mod eslint_probe_request;
pub mod eslint_probe_result;
#[cfg(any(unix, windows))]
mod eslint_project_context;
mod eslint_syntax_evidence;
mod eslint_task_recheck;
#[cfg(any(unix, windows))]
mod eslint_workbench;
pub mod help_command;
#[cfg(any(unix, windows))]
mod npm_audit_arguments;
#[cfg(any(unix, windows))]
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
#[cfg(any(unix, windows))]
mod npm_workspace_scope;
mod platform_fs;
mod python_dependency_discovery;
mod signed_disposition_binding;
#[cfg(any(unix, windows))]
mod swift_syntax_probe;
mod tool_platform;
pub mod tools_command;
pub use bound_false_positive_disposition::BoundFalsePositiveDisposition;
pub use signed_disposition_binding::bind_signed_false_positive_preview;
#[cfg(any(unix, windows))]
pub use signed_disposition_binding::bind_signed_false_positive_replacement_preview;
#[cfg(any(unix, windows))]
mod git_approval_baseline_context;
#[cfg(any(unix, windows))]
mod git_approval_binding;
#[cfg(any(unix, windows))]
pub use git_approval_baseline_context::GitApprovalBaselineContext;
#[cfg(any(unix, windows))]
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
#[cfg(any(unix, windows))]
pub mod check_budget;
#[cfg(any(unix, windows))]
pub mod check_command;
#[cfg(any(unix, windows))]
mod check_eslint_scan;
pub mod check_plan;
pub mod check_request;
mod check_selection;
#[cfg(all(any(unix, windows), feature = "wasm-precheck"))]
mod check_syntax_candidates;
mod checkstyle_preparation;
mod checkstyle_preparation_recheck;
pub mod checkstyle_probe;
pub mod checkstyle_probe_request;
pub mod checkstyle_probe_result;
mod checkstyle_task_recheck;
mod checkstyle_workbench;
#[cfg(any(unix, windows))]
pub mod claude_hook_command;
pub mod config_command;
mod config_project_observation;
pub mod conversation_feedback;
pub mod corpus;
pub mod discovery;
pub mod false_positive_decision;
#[cfg(any(unix, windows))]
pub mod git_index_safety;
#[cfg(any(unix, windows))]
pub mod git_index_safety_command;
#[cfg(any(unix, windows))]
pub mod go_lint_command;
pub mod grammar_evaluation;
mod grammar_evaluation_case;
mod grammar_evaluation_corpus;
#[cfg(feature = "wasm-precheck")]
pub mod grammar_probe_command;
pub mod grammar_route;
pub mod grammar_status_command;
#[cfg(any(unix, windows))]
pub mod hook_execute_command;
#[cfg(any(unix, windows))]
mod hook_fast_scan;
pub mod hook_plan_command;
pub mod init_command;
mod java_checker_config_status;
pub mod java_checkstyle_command;
mod java_cve_attribution;
#[cfg(any(unix, windows))]
mod java_cve_scan;
#[cfg(any(unix, windows))]
mod java_dependency_scan;
#[cfg(any(unix, windows))]
pub mod java_javadoc_command;
mod java_javadoc_scan;
#[cfg(any(unix, windows))]
pub mod java_lint_dispatch;
#[cfg(any(unix, windows))]
pub mod java_p3c_command;
mod java_p3c_scan;
mod java_p3c_workbench;
#[cfg(feature = "wasm-precheck")]
mod java_syntax_precheck;
pub mod legacy_v1_protocol;
#[cfg(any(unix, windows))]
mod maven_dependency_probe;
#[cfg(any(unix, windows))]
mod maven_javadoc_probe;
#[cfg(any(unix, windows))]
pub mod maven_probe;
mod native_tool_candidate;
mod native_tool_discovery;
pub mod next_command;
#[cfg(any(unix, windows))]
mod owasp_maven_probe;
#[cfg(any(unix, windows))]
pub mod partial_sarif_feedback;
pub mod plan_command;
#[cfg(any(unix, windows))]
pub mod pmd6_probe;
#[cfg(any(unix, windows))]
mod project_runtime_options;
mod python_confirmation_recheck;
#[cfg(any(unix, windows))]
pub mod python_cve_command;
pub(crate) mod python_cve_task_recheck;
#[cfg(any(unix, windows))]
pub mod python_lint_command;
#[cfg(any(unix, windows))]
pub mod python_lint_scan;
#[cfg(any(unix, windows))]
mod python_selected_discovery;
#[cfg(any(unix, windows))]
mod python_syntax_confirmation;
#[cfg(any(unix, windows))]
mod python_task_resolution_policy_input;
#[cfg(any(unix, windows))]
mod python_task_resolution_request;
#[cfg(any(unix, windows))]
mod python_task_resolution_service;
#[cfg(any(unix, windows))]
pub use python_confirmation_recheck::validate_python_task_original_source;
#[cfg(any(unix, windows))]
pub use python_task_resolution_request::PythonTaskResolutionRequest;
#[cfg(any(unix, windows))]
pub use python_task_resolution_service::verify_python_task_resolution;
#[cfg(all(any(unix, windows), feature = "wasm-precheck"))]
mod python_syntax_precheck;
pub mod quality_policy_candidate;
#[cfg(any(unix, windows))]
mod report_export;
#[cfg(any(unix, windows))]
pub mod ruff_probe;
#[cfg(any(unix, windows))]
mod ruff_tool_selection;
mod ruff_verification_configuration;
pub mod rules_list_command;
pub mod run_report;
#[cfg(any(unix, windows))]
pub mod rust_build_command;
pub(crate) mod rust_build_task_recheck;
#[cfg(any(unix, windows))]
pub mod rust_comments_command;
pub(crate) mod rust_cve_task_recheck;
#[cfg(any(unix, windows))]
mod rust_lint_inputs;
#[cfg(any(unix, windows))]
pub mod rust_lint_scan;
#[cfg(any(unix, windows))]
mod rust_native_syntax_coverage;
pub mod rustdoc_repair_brief;
#[cfg(any(unix, windows))]
pub(crate) mod rustdoc_task_recheck;
pub mod sarif_feedback;
mod source_language_hint;
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
#[cfg(any(unix, windows))]
pub mod task_attempt_command;
#[cfg(any(unix, windows))]
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
#[cfg(any(unix, windows))]
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

mod native_syntax_confirmation;
#[cfg(any(unix, windows))]
mod syntax_confirmation;

#[cfg(any(unix, windows))]
mod syntax_task_recheck;
#[cfg(any(unix, windows))]
mod zig_syntax_probe;
#[cfg(any(unix, windows))]
mod zig_tool_selection;

#[cfg(any(unix, windows))]
mod plain_syntax_source;

#[cfg(any(unix, windows))]
mod erlang_task_resolution_request;
#[cfg(any(unix, windows))]
mod go_task_resolution_request;
#[cfg(any(unix, windows))]
mod kotlin_task_resolution_request;
#[cfg(any(unix, windows))]
mod swift_task_resolution_request;
#[cfg(any(unix, windows))]
mod syntax_task_resolution_request;
#[cfg(any(unix, windows))]
mod task_lifecycle_store;
#[cfg(any(unix, windows))]
mod task_resolution_checker;
#[cfg(any(unix, windows))]
mod task_resolution_policy_input;
#[cfg(any(unix, windows))]
mod task_resolution_service;
#[cfg(any(unix, windows))]
mod zig_task_resolution_request;
#[cfg(any(unix, windows))]
pub use erlang_task_resolution_request::ErlangTaskResolutionRequest;
#[cfg(any(unix, windows))]
pub use go_task_resolution_request::GoTaskResolutionRequest;
#[cfg(any(unix, windows))]
pub use kotlin_task_resolution_request::KotlinTaskResolutionRequest;
#[cfg(any(unix, windows))]
pub use swift_task_resolution_request::SwiftTaskResolutionRequest;
#[cfg(any(unix, windows))]
pub use task_resolution_service::{
    verify_erlang_task_resolution, verify_go_task_resolution, verify_kotlin_task_resolution,
    verify_swift_task_resolution, verify_zig_task_resolution,
};
#[cfg(any(unix, windows))]
pub use zig_task_resolution_request::ZigTaskResolutionRequest;

#[cfg(any(unix, windows))]
mod task_resolution_evidence_shape;

#[cfg(any(unix, windows))]
pub mod kotlin_lint_command;

#[cfg(any(unix, windows))]
mod kotlin_tool_selection;

#[cfg(any(unix, windows))]
mod check_kotlin_scan;

#[cfg(any(unix, windows))]
pub mod swift_lint_command;
#[cfg(any(unix, windows))]
mod swift_tool_selection;

#[cfg(any(unix, windows))]
mod check_swift_scan;

#[cfg(any(unix, windows))]
mod check_zig_scan;

#[cfg(any(unix, windows))]
mod hook_native_tools;

#[cfg(all(feature = "wasm-precheck", unix))]
pub mod grammar_native_differential;

pub mod syntax_worker_structure;

#[cfg(any(unix, windows))]
mod check_ruby_scan;
#[cfg(any(unix, windows))]
mod go_lint_fallback;
#[cfg(any(unix, windows))]
mod go_syntax_probe;
#[cfg(any(unix, windows))]
mod go_tool_selection;
#[cfg(all(feature = "wasm-precheck", unix))]
mod grammar_native_checker;
#[cfg(all(feature = "wasm-precheck", unix))]
mod javascript_syntax_probe;
#[cfg(any(unix, windows))]
mod python_syntax_probe;
#[cfg(any(unix, windows))]
pub mod ruby_lint_command;
#[cfg(any(unix, windows))]
mod ruby_lint_workbench;
#[cfg(any(unix, windows))]
mod ruby_project_version;
#[cfg(any(unix, windows))]
mod ruby_syntax_probe;
#[cfg(any(unix, windows))]
mod ruby_tool_selection;

#[cfg(any(unix, windows))]
mod rust_lint_arguments;
#[cfg(any(unix, windows))]
pub mod rust_lint_command;
#[cfg(any(unix, windows))]
mod rust_lint_fallback;
#[cfg(any(unix, windows))]
mod rust_lint_workbench;

#[cfg(any(unix, windows))]
mod shell_lint_arguments;
#[cfg(any(unix, windows))]
pub mod shell_lint_command;
#[cfg(any(unix, windows))]
mod shellcheck_config;
#[cfg(any(unix, windows))]
mod shellcheck_probe;
