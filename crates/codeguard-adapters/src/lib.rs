//! 旧语言清单的只读迁移观察；清单不表示原生检查器已实现。

mod legacy_language_identity;

use codeguard_core::{CANDIDATE_PLATFORMS, CHECK_CATEGORIES};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashSet};

mod capability_validation;
mod production_acceptance_plan;
mod python_suite_rule;
mod javascript_binding_rule;
pub use javascript_binding_rule::{javascript_binding_node_kinds, javascript_binding_rule_sha256};
pub use python_suite_rule::{is_required_python_suite_parent, python_suite_rule_sha256};
mod go_package_rule;
pub use go_package_rule::{go_package_rule_sha256, missing_go_package_candidate};
mod cargo_audit;
mod cargo_build;
pub use cargo_audit::{
    CargoAuditFinding, CargoAuditObservation, bind_cargo_audit_lockfile, parse_cargo_audit_json,
};
mod cargo_build_diagnostic;
mod cargo_build_parsed;
pub use cargo_build::parse_cargo_build_json;
pub use cargo_build_diagnostic::CargoBuildDiagnostic;
pub use cargo_build_parsed::CargoBuildParsed;
mod cargo_clippy;
mod cargo_documentation_config;
mod cargo_documentation_workspace;
mod cargo_workspace_reference;
mod cargo_module_model;
mod cargo_edition_declaration;
mod cargo_rustdoc;
mod checker_configuration;
mod checkstyle_command;
mod checkstyle_config;
mod checkstyle_diagnostic;
mod checkstyle_failure;
mod checkstyle_parsed;
mod checkstyle_result;
mod checkstyle_rule_binding;
mod eslint_command;
mod eslint_config_state;
mod eslint_local_candidate;
mod eslint_local_readiness;
mod npm_audit_command;
mod npm_locked_node;
mod rustdoc_finding;
mod rustdoc_parsed;
pub use npm_locked_node::NpmLockedNode;
mod npm_audit_component;
mod npm_audit_config;
pub use npm_audit_command::NpmAuditCommand;
pub use npm_audit_config::inspect_npm_audit_config;
mod grammar_test_corpus;
mod grammar_test_sample;
mod npm_audit_json;
mod npm_audit_observation;
mod source_map;
mod source_mapped_recovery;
mod strict_json;
pub use grammar_test_corpus::parse_grammar_test_corpus;
pub use grammar_test_sample::GrammarTestSample;
mod syntax_precheck_candidate_report;
pub use eslint_config_state::EslintConfigState;
pub use eslint_local_candidate::EslintLocalCandidate;
pub use eslint_local_readiness::inspect_eslint_local_candidate;
pub use npm_audit_component::NpmAuditComponent;
pub use npm_audit_json::parse_npm_audit_json;
pub use npm_audit_observation::NpmAuditObservation;
pub use source_map::{SourceMap, map_syntax_recovery};
pub use source_mapped_recovery::SourceMappedRecovery;
pub use strict_json::parse_unique_json;
pub use syntax_precheck_candidate_report::parse_syntax_precheck_candidate_report;
mod eslint_effective_rule;
pub use eslint_effective_rule::parse_eslint_effective_rule;
mod dart_wasm_compat;
mod eslint_diagnostic;
mod eslint_file_report;
mod eslint_json;
mod eslint_message_report;
mod eslint_parsed;
mod go_candidate;
mod ruby_candidate;
mod ruby_candidate_profile;
mod ruby_candidate_slot;
mod ruby_candidate_tool;
mod grammar_asset_manifest;
mod legacy_dylink_compat;
mod zig_wasm_compat;
pub use dart_wasm_compat::adapt_dart_wasm;
pub use grammar_asset_manifest::{
    GrammarAsset, GrammarAssetManifest, bundled_grammar_candidate, bundled_grammar_candidates,
    bundled_grammar_metadata, parse_grammar_asset_manifest, verify_grammar_asset,
};
pub use legacy_dylink_compat::adapt_legacy_dylink;
pub use zig_wasm_compat::adapt_zig_wasm;
mod go_list_package;
mod go_list_scope;
mod go_vet;
mod javadoc_output;
mod javadoc_replay_pom;
mod maven_dependency_pom;
mod maven_dependency_tree;
mod maven_javadoc_output;
mod maven_module_model;
mod maven_wrapper_candidate;
mod maven_wrapper_readiness;
mod owasp_dependency_check;
mod owasp_maven_attribution;
mod owasp_maven_pom;
mod p3c_rule_catalog;
mod package_manifest;
mod pip_audit_command;
mod pip_audit_dependency;
mod pip_audit_finding;
mod pip_audit_json;
mod pip_audit_observation;
mod pmd6_command;
mod pmd_xml;
mod python_requirements;
mod python_standard_lock;
mod ruff;
mod ruff_documentation_rule;
mod ruff_rulepack;
mod ruff_settings;

pub use capability_validation::validate_capability_inventory;
pub use cargo_clippy::{ClippyFinding, ClippyParsed, parse_cargo_clippy_json};
pub use cargo_documentation_config::inspect_cargo_documentation_config;
pub use cargo_documentation_workspace::inspect_cargo_documentation_workspace;
pub use cargo_workspace_reference::CargoWorkspaceReference;
pub use cargo_module_model::CargoModuleModel;
pub use cargo_edition_declaration::CargoEditionDeclaration;
pub use cargo_rustdoc::parse_cargo_rustdoc_json;
pub use checker_configuration::{
    CheckerConfiguration, configured_p3c_rulesets, inspect_gradle_unknown, inspect_maven_pom,
    inspect_maven_unreadable, inspect_ruff_config, inspect_ruff_missing, inspect_ruff_unknown,
};
pub use checkstyle_command::CheckstyleCommand;
pub use checkstyle_config::checkstyle_comment_config_local_eligible;
pub use checkstyle_config::{checkstyle_comment_rule_bindings, checkstyle_detailed_rule_class};
pub use checkstyle_diagnostic::CheckstyleDiagnostic;
pub use checkstyle_failure::checkstyle_native_failure_reason;
pub use checkstyle_parsed::{CheckstyleParsed, parse_checkstyle_xml};
pub use checkstyle_result::{CheckstyleResult, evaluate_checkstyle_report};
pub use checkstyle_rule_binding::CheckstyleRuleBinding;
pub use eslint_command::EslintCommand;
pub use eslint_diagnostic::EslintDiagnostic;
pub use eslint_json::{eslint_report_version_matches, parse_eslint_json};
pub use eslint_parsed::EslintParsed;
pub use go_candidate::{
    GoCandidateProfile, GoCandidateSlot, bundled_go_candidate_profile, parse_go_candidate_profile,
};
pub use go_list_scope::{GoListScope, parse_go_list_scope};
pub use go_vet::{GoVetFinding, GoVetParseState, GoVetParsed, parse_go_vet_json};
pub use javadoc_output::{
    JavadocDiagnostic, JavadocParseState, JavadocParsed, parse_detailed_javadoc_output,
    parse_javadoc_output,
};
pub use javadoc_replay_pom::javadoc_pom_direct_replay_eligible;
pub use maven_dependency_pom::{dependency_pom_direct_replay_eligible, dependency_pom_project_identity};
pub use maven_dependency_tree::{
    MavenDependencyNode, MavenDependencyTree, parse_maven_dependency_tree_json,
};
pub use maven_javadoc_output::{
    MavenJavadocDiagnostic, MavenJavadocParseState, MavenJavadocParsed,
    parse_detailed_maven_javadoc_output, parse_maven_javadoc_output,
};
pub use maven_module_model::MavenModuleModel;
pub use maven_wrapper_candidate::MavenWrapperCandidate;
pub use maven_wrapper_readiness::inspect_maven_wrapper_candidate;
pub use owasp_dependency_check::{
    OwaspAdvisoryObservation, OwaspDependencyCheckReport, parse_owasp_dependency_check_json,
};
pub use owasp_maven_attribution::{
    OwaspArtifactBindingState, OwaspAttribution, OwaspAttributionState,
    attribute_owasp_advisories_to_maven_graph, bind_owasp_advisories_to_artifact_digests,
};
pub use owasp_maven_pom::{OwaspMavenPomPlan, owasp_maven_pom_plan};
pub use p3c_rule_catalog::p3c_rule_in_selected_rulesets;
pub use package_manifest::{PackageDeclaration, parse_package_json};
pub use pip_audit_command::PipAuditCommand;
pub use pip_audit_dependency::PipAuditDependency;
pub use pip_audit_finding::PipAuditFinding;
pub use pip_audit_json::parse_pip_audit_json;
pub use pip_audit_observation::PipAuditObservation;
pub use pmd_xml::{PmdDiagnostic, PmdParseState, PmdParsed, parse_pmd_xml};
pub use pmd6_command::Pmd6Command;
pub use python_requirements::{PythonRequirementsPins, inspect_python_requirements_pins};
pub use python_standard_lock::{bind_pip_audit_lockfile, parse_pylock_package_identities};
pub use ruff::{
    RuffDiagnostic, RuffLocation, RuffParseState, RuffParsed, is_ruff_pydocstyle_rule,
    parse_ruff_json,
};
pub use ruff_documentation_rule::{RuffDocumentationRule, is_ruff_documentation_rule};
pub use ruff_rulepack::{RuffRuleMapping, RuffRulepack, bundled_ruff_rulepack, parse_ruff_rulepack};
pub use ruff_settings::{RuffSettingsObservation, parse_ruff_settings};
pub use rustdoc_finding::RustdocFinding;
pub use rustdoc_parsed::RustdocParsed;

/// 从旧注册表导入的语言元数据。
#[derive(Debug, Deserialize)]
pub struct Language {
    /// 规范语言 ID。
    pub id: String,
    /// 旧版本宣称的状态，仅用于迁移对照。
    pub status: String,
    /// 文件扩展名，用于后续静态发现。
    #[serde(default)]
    pub extensions: Vec<String>,
    /// 项目清单文件名候选；仅供只读发现，不能证明构建器可执行。
    #[serde(default)]
    pub markers: Vec<String>,
    /// 旧清单登记的原生检查器配置文件名候选；存在不等于检查器已配置。
    #[serde(default)]
    pub linter_config_files: Vec<String>,
    /// 旧清单的 lint 命令声明，不构成新适配器验收。
    pub lint: Option<Vec<String>>,
    /// 旧清单的格式化命令声明，不能替代 lint。
    pub format: Option<Vec<String>>,
}

/// 某语言在候选平台上的六类别能力矩阵。
#[derive(Debug, Serialize)]
pub struct CapabilityRow<'a> {
    /// 规范语言 ID。
    pub language: &'a str,
    /// 仅供迁移对照的旧登记状态。
    pub legacy_status: &'a str,
    /// 旧 lint 命令是否存在，不能据此宣布 implemented。
    pub legacy_lint_declared: bool,
    /// 旧 formatter 是否存在，不能据此宣布 lint implemented。
    pub legacy_formatter_declared: bool,
    /// 平台到类别到能力状态的完整矩阵。
    pub platforms: BTreeMap<&'static str, BTreeMap<&'static str, CapabilityCell>>,
}

/// 单一语言、类别、平台的能力声明与理由。
#[derive(Clone, Debug, Serialize)]
pub struct CapabilityCell {
    /// implemented、gap 或 not_applicable；本阶段只构造 gap。
    pub status: &'static str,
    /// 当前缺口原因，不能作为跳过义务的理由。
    pub reason: &'static str,
}

/// 为一条旧语言构建保守能力记录；尚无真实验收时所有单元保持 gap。
#[must_use]
pub fn capability_row(language: &Language) -> CapabilityRow<'_> {
    let categories: BTreeMap<&'static str, CapabilityCell> = CHECK_CATEGORIES
        .iter()
        .map(|category| {
            (
                *category,
                CapabilityCell {
                    status: "gap",
                    reason: "native_adapter_not_validated",
                },
            )
        })
        .collect();
    let platforms = CANDIDATE_PLATFORMS
        .iter()
        .map(|platform| (*platform, categories.clone()))
        .collect();
    CapabilityRow {
        language: &language.id,
        legacy_status: &language.status,
        legacy_lint_declared: language
            .lint
            .as_ref()
            .is_some_and(|items| !items.is_empty()),
        legacy_formatter_declared: language
            .format
            .as_ref()
            .is_some_and(|items| !items.is_empty()),
        platforms,
    }
}

/// 语言注册表及其基线版本。
#[derive(Debug, Deserialize)]
pub struct LegacyRegistry {
    /// 旧注册表语言记录。
    pub languages: Vec<Language>,
}

/// 解析编译时固定的旧清单并验证 54 stable/3 planned 身份基线。
pub fn legacy_registry() -> Result<LegacyRegistry, String> {
    let raw = include_str!("../../../rulepacks/legacy_languages.json");
    parse_legacy_registry(raw)
}

/// 解析指定注册表字节并核对固定身份；供损坏发行清单反例使用。
pub fn parse_legacy_registry(raw: &str) -> Result<LegacyRegistry, String> {
    let value = parse_unique_json(raw.as_bytes()).map_err(str::to_owned)?;
    if value["version"] != 1 {
        return Err("旧语言清单版本不匹配".into());
    }
    let registry: LegacyRegistry =
        serde_json::from_value(value).map_err(|error| error.to_string())?;
    let ids: HashSet<&str> = registry
        .languages
        .iter()
        .map(|language| language.id.as_str())
        .collect();
    let stable = registry
        .languages
        .iter()
        .filter(|language| language.status == "stable")
        .count();
    let planned = registry
        .languages
        .iter()
        .filter(|language| language.status == "planned")
        .count();
    if registry.languages.len() != 57
        || ids.len() != 57
        || stable != 54
        || planned != 3
        || registry
            .languages
            .iter()
            .any(|language| !legacy_language_identity::matches(&language.id, &language.status))
    {
        return Err(format!(
            "旧语言清单身份不匹配：unique={}, stable={stable}, planned={planned}",
            ids.len()
        ));
    }
    for language in &registry.languages {
        let mut seen = HashSet::new();
        for candidate in &language.linter_config_files {
            if candidate.is_empty()
                || candidate.contains('\\')
                || candidate.contains('*')
                || candidate.contains('?')
                || candidate.chars().any(char::is_control)
                || candidate
                    .split('/')
                    .any(|part| part.is_empty() || matches!(part, "." | ".."))
                || !seen.insert(candidate)
            {
                return Err(format!("{}: 检查器配置文件名无效或重复", language.id));
            }
        }
    }
    Ok(registry)
}

#[cfg(test)]
mod tests {
    use super::{capability_row, legacy_registry, parse_legacy_registry};

    #[test]
    fn baseline_has_all_legacy_entries_without_claiming_adapter_coverage() {
        let registry = legacy_registry().expect("固定语言清单应可解析");
        assert_eq!(registry.languages.len(), 57);
        assert!(
            registry
                .languages
                .iter()
                .any(|language| language.id == "java")
        );
        assert!(
            registry
                .languages
                .iter()
                .any(|language| language.id == "metal")
        );
    }

    #[test]
    fn registry_rejects_broad_or_traversing_checker_config_names() {
        let baseline: serde_json::Value =
            serde_json::from_str(include_str!("../../../rulepacks/legacy_languages.json")).unwrap();
        for bad in ["../.eslintrc", "src/*.xml", ".", "", "config\\lint.xml"] {
            let mut changed = baseline.clone();
            changed["languages"][0]["linter_config_files"] = serde_json::json!([bad]);
            assert!(
                parse_legacy_registry(&changed.to_string()).is_err(),
                "{bad:?}"
            );
        }
    }

    #[test]
    fn formatter_only_entries_remain_lint_gaps_on_every_platform() {
        let registry = legacy_registry().expect("固定语言清单应可解析");
        for id in ["julia", "pascal"] {
            let language = registry
                .languages
                .iter()
                .find(|language| language.id == id)
                .expect("旧清单必须保留该语言");
            let row = capability_row(language);
            assert!(!row.legacy_lint_declared);
            assert!(row.legacy_formatter_declared);
            assert_eq!(row.platforms.len(), 5);
            for categories in row.platforms.values() {
                assert_eq!(categories.len(), 6);
                assert_eq!(categories["lint"].status, "gap");
            }
        }
    }

    #[test]
    fn corrupt_release_registry_is_not_accepted() {
        assert!(parse_legacy_registry("not json").is_err());
        assert!(parse_legacy_registry(r#"{"languages":[]}"#).is_err());
    }
}
mod cargo_build_finding_identity;
pub use cargo_build_finding_identity::cargo_build_finding_record;
mod eslint_finding_identity;
pub use eslint_finding_identity::{project_eslint_findings, validate_records};
mod go_finding_identity;
pub use go_finding_identity::go_finding_record;
mod rustdoc_finding_identity;
pub use rustdoc_finding_identity::rustdoc_finding_record;

mod kotlin_diagnostic;
mod kotlin_diagnostics;
mod kotlin_parsed;
pub use kotlin_diagnostic::KotlinDiagnostic;
pub use kotlin_diagnostics::parse_kotlin_diagnostics;
pub use kotlin_parsed::KotlinParsed;

mod kotlin_native_observation;
pub use kotlin_native_observation::valid_kotlin_native_observation;

pub use ruby_candidate::{bundled_ruby_candidate_profile, parse_ruby_candidate_profile};
pub use ruby_candidate_profile::RubyCandidateProfile;
pub use ruby_candidate_slot::RubyCandidateSlot;
pub use ruby_candidate_tool::RubyCandidateTool;

mod shellcheck_diagnostic;
mod shellcheck_parsed;
mod shellcheck_json;
pub use shellcheck_diagnostic::ShellCheckDiagnostic;
pub use shellcheck_parsed::ShellCheckParsed;
pub use shellcheck_json::parse_shellcheck_json1;

mod cfquery_projection_rule;
pub use cfquery_projection_rule::{cfquery_projection_rule_sha256, cfquery_projection_span_valid};

mod c_family_preprocessor_guard;
pub use c_family_preprocessor_guard::has_c_family_preprocessor_directive;
mod clang_sarif;
pub use clang_sarif::parse_clang_stdin_sarif;
mod clang_documentation_rule;
pub use clang_documentation_rule::clang_documentation_guidance;
mod clang_documentation_ast;
pub use clang_documentation_ast::parse_clang_documentation_ast;
mod clang_documentation_structure;
pub use clang_documentation_structure::valid_clang_documentation_structure;

mod erlang_form_rule;
pub use erlang_form_rule::erlang_form_rule_sha256;

mod javascript_module_return_rule;
pub use javascript_module_return_rule::{
    javascript_module_return_node_kinds, javascript_module_return_rule_sha256,
};

mod gradle_checker_task;
mod gradle_project_checker_model;
mod gradle_checker_model;
mod gradle_checker_model_parser;
pub use gradle_checker_task::GradleCheckerTask;
pub use gradle_project_checker_model::GradleProjectCheckerModel;
pub use gradle_checker_model::GradleCheckerModel;
pub use gradle_checker_model_parser::parse_gradle_checker_model;

mod gradle_javadoc_task_plan;
mod gradle_javadoc_plan;
mod gradle_javadoc_diagnostic;
mod gradle_javadoc_output;
pub use gradle_javadoc_task_plan::GradleJavadocTaskPlan;
pub use gradle_javadoc_plan::plan_gradle_javadoc_tasks;
pub use gradle_javadoc_diagnostic::GradleJavadocDiagnostic;
pub use gradle_javadoc_output::parse_gradle_javadoc_output;

pub use production_acceptance_plan::parse_production_acceptance_plan;

mod gradle_dependency_check_task_plan;
mod gradle_dependency_check_plan;
pub use gradle_dependency_check_task_plan::GradleDependencyCheckTaskPlan;
pub use gradle_dependency_check_plan::plan_gradle_dependency_check_tasks;

mod gradle_owasp_report_ownership;
mod gradle_owasp_report_ownership_parser;
pub use gradle_owasp_report_ownership::GradleOwaspReportOwnership;
pub use gradle_owasp_report_ownership_parser::parse_gradle_owasp_report_ownership;

mod c_family_compilation_entry;
pub use c_family_compilation_entry::{CFamilyCompilationEntry, parse_c_family_compilation_database};
