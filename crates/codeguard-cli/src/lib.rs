//! CLI 共享的 Rust 开发期验收与命令应用服务入口。

#[cfg(unix)]
pub mod gradle_javadoc_probe;
#[cfg(unix)]
pub mod gradle_javadoc_task_recheck;
#[cfg(unix)]
pub mod gradle_javadoc_workbench;
#[cfg(unix)]
pub mod gradle_model_probe;
#[cfg(unix)]
pub mod gradle_model_probe_request;

pub mod agents_block;
pub mod approval_snapshot;
mod bound_false_positive_disposition;
#[cfg(unix)]
mod cargo_tool_selection;
#[cfg(unix)]
mod check_erlang_scan;
mod command_catalogue;
mod command_descriptor;
mod command_examples;
#[cfg(unix)]
pub mod doctor_command;
#[cfg(unix)]
mod doctor_scratch;
#[cfg(unix)]
mod erlang_lint_arguments;
#[cfg(unix)]
pub mod erlang_lint_command;
#[cfg(unix)]
mod erlang_syntax_probe;
#[cfg(unix)]
mod erlang_tool_selection;
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
#[cfg(unix)]
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
pub mod help_command;
pub mod language_alias;
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
#[cfg(unix)]
mod swift_syntax_probe;
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
#[cfg(unix)]
mod check_eslint_scan;
pub mod check_plan;
pub mod check_request;
mod check_selection;
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
mod config_project_observation;
pub mod conversation_feedback;
pub mod corpus;
mod csharp_dependency_scan;
pub mod discovery;
pub mod false_positive_decision;
#[cfg(unix)]
pub mod git_index_safety;
#[cfg(unix)]
pub mod git_index_safety_command;
mod go_dependency_scan;
#[cfg(unix)]
pub mod go_lint_command;
pub mod grammar_evaluation;
mod grammar_evaluation_case;
mod grammar_evaluation_corpus;
#[cfg(feature = "wasm-precheck")]
pub mod grammar_probe_command;
pub mod grammar_route;
pub mod grammar_status_command;
#[cfg(unix)]
pub mod hook_execute_command;
#[cfg(unix)]
mod hook_fast_scan;
pub mod hook_plan_command;
pub mod init_command;
mod java_checker_config_status;
pub mod java_checkstyle_command;
#[cfg(unix)]
pub mod java_comments_command;
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
mod java_p3c_workbench;
#[cfg(feature = "wasm-precheck")]
mod java_syntax_precheck;
#[cfg(unix)]
mod javadoc_task_recheck;
#[cfg(unix)]
mod javadoc_workbench;
pub mod legacy_v1_protocol;
#[cfg(unix)]
mod maven_dependency_probe;
#[cfg(unix)]
mod maven_javadoc_probe;
#[cfg(unix)]
mod maven_javadoc_task_recheck;
#[cfg(unix)]
mod maven_javadoc_workbench;
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
mod python_confirmation_recheck;
#[cfg(unix)]
pub mod python_cve_command;
pub(crate) mod python_cve_task_recheck;
#[cfg(unix)]
pub mod python_lint_command;
#[cfg(unix)]
pub mod python_lint_scan;
#[cfg(unix)]
mod python_selected_discovery;
#[cfg(unix)]
mod python_syntax_confirmation;
#[cfg(unix)]
mod python_task_resolution_policy_input;
#[cfg(unix)]
mod python_task_resolution_request;
#[cfg(unix)]
mod python_task_resolution_service;
mod vbnet_dependency_scan;
#[cfg(unix)]
pub use python_confirmation_recheck::validate_python_task_original_source;
#[cfg(unix)]
pub use python_task_resolution_request::PythonTaskResolutionRequest;
#[cfg(unix)]
pub use python_task_resolution_service::verify_python_task_resolution;
#[cfg(unix)]
mod check_rust_syntax_scan;
#[cfg(all(unix, feature = "wasm-precheck"))]
mod javascript_syntax_precheck;
#[cfg(all(unix, feature = "wasm-precheck"))]
mod python_syntax_precheck;
pub mod quality_policy_candidate;
#[cfg(unix)]
mod report_export;
#[cfg(unix)]
pub mod ruff_probe;
#[cfg(unix)]
mod ruff_tool_selection;
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
mod rust_documentation_command;
#[cfg(unix)]
mod rust_input_inventory;
#[cfg(unix)]
mod rust_lint_inputs;
#[cfg(unix)]
pub mod rust_lint_scan;
#[cfg(unix)]
mod rust_native_syntax_coverage;
#[cfg(unix)]
pub mod rust_project_edition;
#[cfg(unix)]
pub mod rust_project_syntax;
#[cfg(unix)]
mod rust_syntax_evidence;
#[cfg(unix)]
mod rust_syntax_task_recheck;
pub mod rustdoc_repair_brief;
#[cfg(unix)]
pub(crate) mod rustdoc_task_recheck;
#[cfg(unix)]
mod rustfmt_scratch;
#[cfg(unix)]
mod rustfmt_syntax_probe;
#[cfg(unix)]
mod rustfmt_tool_selection;
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
#[cfg(unix)]
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
#[cfg(unix)]
mod syntax_confirmation;

#[cfg(unix)]
mod syntax_task_recheck;
#[cfg(unix)]
mod zig_syntax_probe;
#[cfg(unix)]
mod zig_tool_selection;

#[cfg(unix)]
mod plain_syntax_source;

#[cfg(unix)]
mod erlang_task_resolution_request;
#[cfg(unix)]
mod go_task_resolution_request;
#[cfg(unix)]
mod kotlin_task_resolution_request;
mod ruby_task_resolution_request;
#[cfg(unix)]
mod rust_task_resolution_request;
#[cfg(unix)]
mod swift_task_resolution_request;
#[cfg(unix)]
mod syntax_task_resolution_request;
#[cfg(unix)]
mod task_lifecycle_store;
#[cfg(unix)]
mod task_resolution_checker;
#[cfg(unix)]
mod task_resolution_policy_input;
#[cfg(unix)]
mod task_resolution_service;
#[cfg(unix)]
mod zig_task_resolution_request;
#[cfg(unix)]
pub use erlang_task_resolution_request::ErlangTaskResolutionRequest;
#[cfg(unix)]
pub use go_task_resolution_request::GoTaskResolutionRequest;
#[cfg(unix)]
pub use kotlin_task_resolution_request::KotlinTaskResolutionRequest;
pub use ruby_task_resolution_request::RubyTaskResolutionRequest;
#[cfg(unix)]
pub use rust_task_resolution_request::RustTaskResolutionRequest;
#[cfg(unix)]
pub use swift_task_resolution_request::SwiftTaskResolutionRequest;
#[cfg(unix)]
pub use task_resolution_service::{
    verify_erlang_task_resolution, verify_go_task_resolution, verify_kotlin_task_resolution,
    verify_ruby_task_resolution, verify_rust_task_resolution, verify_swift_task_resolution,
    verify_zig_task_resolution,
};
#[cfg(unix)]
pub use zig_task_resolution_request::ZigTaskResolutionRequest;

#[cfg(unix)]
mod task_resolution_evidence_shape;

#[cfg(unix)]
pub mod kotlin_lint_command;

#[cfg(unix)]
mod kotlin_tool_selection;

#[cfg(unix)]
mod check_kotlin_scan;

#[cfg(unix)]
pub mod swift_lint_command;
#[cfg(unix)]
mod swift_tool_selection;

#[cfg(unix)]
mod check_swift_scan;

#[cfg(unix)]
mod check_zig_scan;

#[cfg(unix)]
mod hook_native_tools;

#[cfg(all(feature = "wasm-precheck", unix))]
pub mod grammar_native_differential;

pub mod syntax_worker_structure;

mod agents_block_merge;
mod architecture_md;
mod architecture_profile;
mod archive_bundle;
mod c_dependency_scan;
mod check_arkts_native_scan;
mod check_c_native_scan;
mod check_cfml_native_scan;
mod check_cfquery_native_scan;
mod check_cfscript_native_scan;
mod check_cobol_native_scan;
mod check_cpp_native_scan;
mod check_csharp_scan;
mod check_dart_scan;
mod check_elixir_scan;
mod check_erlang_native_scan;
mod check_go_scan;
mod check_go_syntax_scan;
mod check_java_scan;
mod check_javascript_native_scan;
mod check_kotlin_native_scan;
mod check_lua_scan;
mod check_luau_native_scan;
mod check_native_results;
mod check_nix_scan;
mod check_objc_scan;
mod check_pascal_scan;
mod check_php_scan;
mod check_python_native_scan;
mod check_r_scan;
mod check_report_assembly;
mod check_ruby_native_scan;
#[cfg(unix)]
mod check_ruby_scan;
mod check_rust_native_scan;
mod check_scala_scan;
mod check_solidity_scan;
mod check_swift_native_scan;
mod check_terraform_scan;
mod check_tsx_native_scan;
mod check_typescript_native_scan;
mod check_vbnet_scan;
mod check_zig_native_scan;
mod ci_release;
mod controlled_fix;
mod cpp_dependency_scan;
mod dart_dependency_scan;
mod disposition_attribution;
mod distribution_installer;
mod elixir_dependency_scan;
mod erlang_dependency_scan;
mod false_positive_investigation;
#[cfg(unix)]
mod go_lint_fallback;
mod go_project_version;
#[cfg(unix)]
mod go_syntax_probe;
#[cfg(unix)]
mod go_tool_selection;
#[cfg(all(feature = "wasm-precheck", unix))]
mod grammar_native_checker;
mod holdout_performance;
mod host_entry;
mod init_readiness;
mod init_transaction;
mod javascript_dependency_scan;
#[cfg(all(feature = "wasm-precheck", unix))]
mod javascript_syntax_probe;
mod kotlin_dependency_scan;
mod lua_dependency_scan;
mod manifest_lock_observer;
mod manifest_refresh;
mod mcp_integration;
mod nix_dependency_scan;
mod objc_dependency_scan;
mod pascal_dependency_scan;
mod php_dependency_scan;
mod plugin_api;
mod policy_repair_test;
mod privacy_verification;
mod project_boundary;
mod python_dependency_scan;
#[cfg(unix)]
mod python_syntax_probe;
mod quality_config_mapping;
mod quality_evaluation;
mod r_dependency_scan;
mod ruby_dependency_scan;
#[cfg(unix)]
pub mod ruby_lint_command;
#[cfg(unix)]
mod ruby_lint_workbench;
#[cfg(unix)]
mod ruby_project_version;
#[cfg(unix)]
mod ruby_syntax_probe;
#[cfg(unix)]
mod ruby_tool_selection;
mod rust_dependency_scan;
mod scala_dependency_scan;
mod schema_freeze;
mod solidity_dependency_scan;
mod status_disposition;
mod swift_dependency_scan;
mod terraform_dependency_scan;
mod token_generation;
mod typed_module_graph;
mod typescript_dependency_scan;
mod zig_dependency_scan;

#[cfg(unix)]
mod rust_lint_arguments;
#[cfg(unix)]
pub mod rust_lint_command;
#[cfg(unix)]
mod rust_lint_fallback;
#[cfg(unix)]
mod rust_lint_workbench;

#[cfg(unix)]
mod shell_lint_arguments;
#[cfg(unix)]
pub mod shell_lint_command;
#[cfg(unix)]
mod shellcheck_config;
#[cfg(unix)]
mod shellcheck_probe;

#[cfg(unix)]
mod shell_lint_workbench;

#[cfg(unix)]
mod shell_task_recheck;

#[cfg(unix)]
mod check_shell_scan;

#[cfg(unix)]
mod clippy_hook_feedback;

mod shell_resolution_evidence;
mod shell_task_resolution_policy_input;
mod shell_task_resolution_request;
mod shell_task_resolution_service;
pub use shell_task_resolution_request::ShellTaskResolutionRequest;
pub use shell_task_resolution_service::verify_shell_task_resolution;

#[cfg(unix)]
mod syntax_lint_arguments;
#[cfg(unix)]
pub mod syntax_lint_command;

#[cfg(all(unix, feature = "wasm-precheck"))]
mod syntax_lint_feedback;

#[cfg(unix)]
mod c_family_comments_arguments;
#[cfg(unix)]
pub mod c_family_comments_command;
#[cfg(unix)]
mod c_family_comments_task_recheck;
#[cfg(unix)]
mod c_family_comments_workbench;
#[cfg(unix)]
mod c_family_structure_workbench;
#[cfg(unix)]
mod clang_lint_feedback;
#[cfg(unix)]
mod clang_syntax_probe;
#[cfg(unix)]
mod native_clang_profile;

#[cfg(feature = "wasm-precheck")]
mod syntax_worker_mode;

pub mod javascript_mode_observation;

pub mod production_acceptance_plan_command;

#[cfg(unix)]
pub mod gradle_dependency_check_request;

#[cfg(unix)]
pub mod gradle_dependency_check_probe;

#[cfg(unix)]
pub mod java_gradle_cve_command;

#[cfg(unix)]
mod gradle_module_cache;

#[cfg(unix)]
mod gradle_owasp_report_budget;

#[cfg(unix)]
pub mod gradle_cve_workbench;

#[cfg(unix)]
pub mod gradle_cve_task_recheck;

#[cfg(unix)]
mod python_comments_arguments;
#[cfg(unix)]
pub mod python_comments_command;

#[cfg(unix)]
mod python_documentation_configuration;

#[cfg(unix)]
mod cargo_audit_database_snapshot;

#[cfg(all(test, unix))]
mod c_family_structure_recheck_tests;

mod c_family_structure_task_recheck;

mod check_c_family_comments;

mod c_family_documentation_hook_feedback;
mod c_family_documentation_host_guidance;

mod c_family_compilation_discovery;

mod c_family_placeholder_workbench;

mod c_family_placeholder_task_recheck;
pub mod run_config;
pub mod baseline_classification;
pub mod init_dry_run;
pub mod artifact_scope;
pub mod finding_identity;
