//! 项目范围内 Swift 原生优先观察；仅调用冻结单文件 frontend parse。
use crate::{plain_syntax_source::read_plain_source, swift_tool_selection::SwiftToolSelection};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeSet,
    path::{Path, PathBuf},
    sync::atomic::{AtomicBool, Ordering},
    time::Instant,
};

/// 观察限定相对路径；共享截止时间和取消状态，最多64文件，不运行项目脚本。
pub(crate) fn observe(
    root: &Path,
    paths: &BTreeSet<String>,
    tool: Option<PathBuf>,
    deadline: Instant,
    cancelled: &AtomicBool,
) -> Value {
    let selection = SwiftToolSelection::discover(tool);
    let mut report = json!({"schema_version":"0.1.0","report_type":"swift_parse_scan","scope":"single_frozen_swift_files","tool_selection":selection.report(),"source_file_count":paths.len(),"unobserved_count":paths.len().saturating_sub(64),"scope_stable":true,"local_parse_complete":false,"files":[],"task_status":"not_connected","authority":"local_unverified","coverage_proven":false,"delivery_decision":"not_evaluated"});
    let mut files = Vec::new();
    for relative in paths.iter().take(64) {
        let mut file = json!({"path":relative,"source_sha256":null,"current":false,"tool_path":selection.tool().and_then(|p|p.canonicalize().ok()),"native":crate::swift_lint_command::unavailable("swift_tool_not_found"),"task_id":null,"task_sync_reason":null,"recheck_argv":null,"next_action":"读取候选初检；疑似异常需要原生确认，完整零恢复时推荐准备原生工具"});
        if cancelled.load(Ordering::Relaxed) || codeguard_runtime::sigint_cancellation_requested() {
            file["native"] = crate::swift_lint_command::unavailable("request_cancelled");
            files.push(file);
            continue;
        }
        if Instant::now() >= deadline {
            file["native"] = crate::swift_lint_command::unavailable("request_deadline_exceeded");
            files.push(file);
            continue;
        }
        let source = match read_plain_source(&root.join(relative)) {
            Ok(s) => s,
            Err(_) => {
                file["native"] =
                    crate::swift_lint_command::unavailable("swift_source_unavailable_or_invalid");
                files.push(file);
                continue;
            }
        };
        file["source_sha256"] = json!(digest(&source));
        file["current"] = json!(true);
        if let Some(tool) = selection.tool() {
            file["native"] = crate::swift_syntax_probe::observe(tool, &source, deadline);
            let has_syntax = file["native"]["diagnostics"]
                .as_array()
                .is_some_and(|r| !r.is_empty());
            file["next_action"] = json!(if has_syntax {
                "按当前原生 UTF-8 字节列修复语法；项目语义另行核对依赖，继续完整项目检查"
            } else if file["native"]["status"] == "completed" {
                "本文件原生零诊断；继续项目 lint、类型、注释、安全与依赖检查，不凭此关闭任务"
            } else {
                "核对选定 Apple Swift 6.4、SDK、上下文和预算后原工具复检；不改无关源码、不换工具绕过失败"
            });
            if file["native"]["version"] == "Apple Swift 6.4" {
                file["recheck_argv"] = json!([
                    "codeguard",
                    "lint",
                    "swift",
                    root.join(relative),
                    "--swift-tool",
                    tool,
                    "--format=json"
                ]);
            }
        }
        files.push(file);
    }
    report["files"] = json!(files);
    refresh(root, &mut report, deadline);
    report
}
/// 复核每份原生观察的源码、入口目标和 launcher 字节；失效时撤回所有定位。
pub(crate) fn refresh(root: &Path, report: &mut Value, deadline: Instant) {
    let selected = report["tool_selection"]["executable"]
        .as_str()
        .map(PathBuf::from);
    let resolved = selected.as_ref().and_then(|p| p.canonicalize().ok());
    let sha = resolved
        .as_ref()
        .and_then(|p| codeguard_runtime::read_bounded_regular_file(p, 64 * 1024 * 1024).ok())
        .map(|b| digest(&b));
    for file in report["files"].as_array_mut().into_iter().flatten() {
        if file["current"] != true {
            continue;
        }
        let reason = if Instant::now() >= deadline {
            Some("request_deadline_exceeded")
        } else if file["tool_path"]
            .as_str()
            .is_some_and(|p| resolved.as_deref() != Some(Path::new(p)))
            || file["native"]["tool_sha256"]
                .as_str()
                .is_some_and(|s| sha.as_deref() != Some(s))
        {
            Some("swift_tool_changed_after_check")
        } else if file["path"]
            .as_str()
            .and_then(|p| read_plain_source(&root.join(p)).ok())
            .is_none_or(|b| file["source_sha256"] != digest(&b))
        {
            Some("swift_source_changed_after_check")
        } else {
            None
        };
        if let Some(reason) = reason {
            file["current"] = json!(false);
            file["native"] = crate::swift_lint_command::unavailable(reason);
            file["recheck_argv"] = Value::Null;
            file["next_action"] = json!("原观察已失效；先复检当前输入，不沿用旧位置修补源码");
        }
    }
    report["local_parse_complete"] = json!(
        report["scope_stable"] == true
            && report["unobserved_count"] == 0
            && report["files"].as_array().is_some_and(|r| !r.is_empty()
                && r.iter().all(|f| f["current"] == true
                    && matches!(
                        f["native"]["status"].as_str(),
                        Some("completed" | "diagnostics_observed")
                    )))
    );
}
/// 已选择原生入口的文件不再改走 WASM；该偏好不证明原生检查成功或覆盖完整。
#[cfg(feature = "wasm-precheck")]
pub(crate) fn prefers(report: &Value, relative: &str) -> bool {
    report["tool_selection"]["source"]
        .as_str()
        .is_some_and(|s| s != "not_found")
        && relative.ends_with(".swift")
        && (report["files"]
            .as_array()
            .is_some_and(|r| r.iter().any(|f| f["path"] == relative))
            || report["unobserved_count"].as_u64().is_some_and(|n| n > 0))
}
fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

/// SwiftLint 规则配置的候选位置；工具在运行目录查找默认配置，两个扩展名均接受。
const RULE_CONFIG_CANDIDATES: [&str; 2] = [".swiftlint.yml", ".swiftlint.yaml"];
/// 配置文件读取预算；超出即无法完整判定，不猜测内容。
const RULE_CONFIG_BUDGET: u64 = 1024 * 1024;

/// Swift 六类别适用性研究档案（OpenSpec 8.10）：只登记权威候选、方言、版本策略与缺口。
/// 该档案不运行任何工具、不读取 Package.swift/pbxproj、不授予任何类别能力；缺工具是
/// 准备缺口，不是 not_applicable。发布到 scan/lint 报告需先扩展对应 JSON Schema（共享文件）。
#[cfg_attr(not(test), allow(dead_code))]
pub(crate) fn applicability_profile() -> Value {
    json!({
        "schema_version": "0.1.0",
        "language": "swift",
        "dialects": ["swift"],
        "dialect_validation": "unverified",
        "platforms": codeguard_core::CANDIDATE_PLATFORMS,
        "platform_validation": "unverified",
        "target_version_inputs": [
            "Package.swift",
            "Package.resolved",
            ".swift-version",
            ".tool-versions",
            "project.yml"
        ],
        "version_gaps": [
            "apple_swift_6_4_single_file_parse_does_not_qualify_other_toolchains_or_versions",
            "xcode_build_system_schemes_and_destinations_require_project_model_without_running_scripts",
            "swiftlint_and_docc_plugin_versions_require_project_lock_binding"
        ],
        "categories": [
            {
                "category": "lint",
                "applicability": "applicable",
                "status": "gap",
                "tools": [{
                    "tool_id": "swiftlint",
                    "candidate_version": "project_locked",
                    "version_status": "unvalidated_project_lock_required",
                    "candidate_argv": ["swiftlint", "lint", "--reporter", "json"],
                    "scope": "swift_project_with_swiftlint_config_or_defaults",
                    "sources": [
                        "https://github.com/realm/SwiftLint/blob/main/README.md",
                        "https://github.com/realm/SwiftLint/blob/main/Source/SwiftLint/SupportingFiles/SwiftLint.yaml"
                    ]
                }],
                "requires_database_freshness": false,
                "coverage_gaps": [
                    "swift_frontend_single_file_parse_observes_syntax_not_project_style_lint",
                    "swiftlint_effective_configuration_and_json_reporter_contract_unvalidated",
                    "nested_and_parent_config_resolution_requires_tool_context"
                ]
            },
            {
                "category": "comments",
                "applicability": "applicable",
                "status": "gap",
                "tools": [{
                    "tool_id": "swift_docc",
                    "candidate_version": "project_locked",
                    "version_status": "unvalidated_project_lock_required",
                    "candidate_argv": ["swift", "package", "generate-documentation"],
                    "scope": "swift_package_with_docc_plugin",
                    "sources": ["https://www.swift.org/documentation/docc/"]
                }],
                "requires_database_freshness": false,
                "coverage_gaps": [
                    "docc_diagnoses_malformed_comments_not_presence_or_param_return_throws_completeness",
                    "docc_plugin_presence_and_symbol_graph_generation_unverified",
                    "no_official_presence_rule_candidate_documented_in_swiftlint_core_rules"
                ]
            },
            {
                "category": "dependencies",
                "applicability": "applicable",
                "status": "gap",
                "tools": [{
                    "tool_id": "swift_package_resolve",
                    "candidate_version": "project_locked",
                    "version_status": "unvalidated_project_lock_required",
                    "candidate_argv": ["swift", "package", "resolve"],
                    "scope": "swift_package_manager_project_with_package_resolved",
                    "sources": ["https://www.swift.org/documentation/package-manager/"]
                }],
                "requires_database_freshness": false,
                "coverage_gaps": [
                    "package_resolved_v1_and_v2_lock_binding_not_verified",
                    "resolve_executes_spm_and_requires_no_project_script_side_effect_policy",
                    "licenses_sbom_and_provenance_sources_unvalidated"
                ]
            },
            {
                "category": "cve",
                "applicability": "applicable",
                "status": "gap",
                "tools": [{
                    "tool_id": "osv_scanner_swift",
                    "candidate_version": "project_locked",
                    "version_status": "unvalidated_project_lock_required",
                    "candidate_argv": ["osv-scanner", "--lockfile", "Package.resolved"],
                    "scope": "swift_project_with_package_resolved_no_official_spm_advisory_scanner",
                    "sources": ["https://google.github.io/osv-scanner/supported_lockfiles/"]
                }],
                "requires_database_freshness": true,
                "coverage_gaps": [
                    "osv_database_snapshot_identity_and_freshness_must_be_bound",
                    "network_access_must_be_refused_before_offline_policy",
                    "third_party_scanner_is_not_an_official_swift_ecosystem_tool"
                ]
            },
            {
                "category": "security",
                "applicability": "applicable",
                "status": "gap",
                "tools": [{
                    "tool_id": "semgrep_swift",
                    "candidate_version": "project_locked",
                    "version_status": "unvalidated_project_lock_required",
                    "candidate_argv": ["semgrep", "scan", "--config", "<approved-ruleset>", "--json"],
                    "scope": "swift_experimental_support_no_official_swift_sast",
                    "sources": ["https://docs.semgrep.dev/docs/supported-languages/"]
                }],
                "requires_database_freshness": false,
                "coverage_gaps": [
                    "swift_support_is_experimental_and_may_fall_back_to_generic_mode",
                    "third_party_ruleset_selection_and_provenance_unbound",
                    "swiftlint_has_no_security_ruleset_so_no_official_candidate_exists"
                ]
            },
            {
                "category": "build",
                "applicability": "project_dependent",
                "status": "gap",
                "tools": [
                    {
                        "tool_id": "swift_build",
                        "candidate_version": "project_locked",
                        "version_status": "unvalidated_project_lock_required",
                        "candidate_argv": ["swift", "build"],
                        "scope": "swift_package_manager_project",
                        "sources": ["https://www.swift.org/documentation/package-manager/"]
                    },
                    {
                        "tool_id": "xcodebuild",
                        "candidate_version": "project_locked",
                        "version_status": "unvalidated_project_lock_required",
                        "candidate_argv": ["xcodebuild", "-scheme", "<selected>", "build"],
                        "scope": "xcode_project_or_workspace",
                        "sources": ["https://developer.apple.com/documentation/xcode/building-from-the-command-line-with-xcodebuild"]
                    }
                ],
                "requires_database_freshness": false,
                "coverage_gaps": [
                    "build_applicability_depends_on_project_build_target",
                    "scheme_destination_and_signing_selection_require_project_model",
                    "build_execution_requires_explicit_no_script_side_effect_policy"
                ]
            }
        ]
    })
}

/// 只读发现项目 SwiftLint 规则配置并区分 configured/missing/invalid/unknown；
/// 与其它观察共用截止时间和取消令牌，不运行 swiftlint/swift、不安装工具。
/// `configured` 只表示文件存在且结构上是映射形态的 YAML 子集，不证明规则语义或工具可用。
#[cfg_attr(not(test), allow(dead_code))]
pub(crate) fn observe_rule_config(root: &Path, deadline: Instant, cancelled: &AtomicBool) -> Value {
    let mut report = json!({
        "schema_version": "0.1.0",
        "report_type": "swift_swiftlint_rule_config",
        "tool_id": "swiftlint",
        "candidates": RULE_CONFIG_CANDIDATES,
        "found_candidates": [],
        "selected": null,
        "state": "unknown",
        "reason": null,
        "config_sha256": null,
        "size_bytes": null,
        "semantics": "unresolved",
        "tool_execution": "not_attempted"
    });
    let existing: Vec<&str> = RULE_CONFIG_CANDIDATES
        .iter()
        .filter(|candidate| root.join(candidate).symlink_metadata().is_ok())
        .copied()
        .collect();
    report["found_candidates"] = json!(existing);
    let prelude = |report: &mut Value, reason: &str| {
        report["state"] = json!("unknown");
        report["reason"] = json!(reason);
    };
    if cancelled.load(Ordering::Relaxed) || codeguard_runtime::sigint_cancellation_requested() {
        prelude(&mut report, "request_cancelled");
        return report;
    }
    if Instant::now() >= deadline {
        prelude(&mut report, "request_deadline_exceeded");
        return report;
    }
    match existing.as_slice() {
        [] => {
            report["state"] = json!("missing");
            report["reason"] = json!("rule_config_not_found");
        }
        [selected] => {
            report["selected"] = json!(selected);
            classify_rule_config(root, selected, &mut report);
        }
        _ => prelude(&mut report, "rule_config_candidate_ambiguous"),
    }
    report
}

/// 复核既有配置观察是否仍然当前；输入变化必须显式失效，不沿用旧摘要。
#[cfg_attr(not(test), allow(dead_code))]
pub(crate) fn rule_config_recheck(root: &Path, report: &Value) -> Option<&'static str> {
    let Some(selected) = report["selected"].as_str() else {
        // 缺配置观察也要复核：候选后来出现即视为输入变化。
        return RULE_CONFIG_CANDIDATES
            .iter()
            .any(|candidate| root.join(candidate).symlink_metadata().is_ok())
            .then_some("rule_config_now_present_after_check");
    };
    let path = root.join(selected);
    let metadata = path.symlink_metadata().ok();
    if metadata
        .as_ref()
        .is_some_and(|meta| !meta.file_type().is_symlink() && meta.file_type().is_file())
    {
        if let Ok(bytes) = codeguard_runtime::read_bounded_regular_file(&path, RULE_CONFIG_BUDGET) {
            if report["config_sha256"]
                .as_str()
                .is_some_and(|previous| previous == digest(&bytes))
            {
                return None;
            }
            return Some("rule_config_changed_after_check");
        }
        return Some("rule_config_read_failed_after_check");
    }
    if metadata.is_none() {
        return Some("rule_config_removed_after_check");
    }
    Some("rule_config_unusable_after_check")
}

/// 逐项给出可证伪的坏配置理由；无法证明损坏时保持 unknown，不猜语义。
fn classify_rule_config(root: &Path, selected: &str, report: &mut Value) {
    let path = root.join(selected);
    let Ok(metadata) = path.symlink_metadata() else {
        report["reason"] = json!("rule_config_read_failed");
        return;
    };
    if metadata.file_type().is_symlink() {
        report["reason"] = json!("rule_config_symlink_not_followed");
        return;
    }
    if !metadata.file_type().is_file() {
        report["state"] = json!("invalid");
        report["reason"] = json!("rule_config_not_a_regular_file");
        return;
    }
    if metadata.len() > RULE_CONFIG_BUDGET {
        report["reason"] = json!("rule_config_budget_exceeded");
        return;
    }
    let Ok(bytes) = codeguard_runtime::read_bounded_regular_file(&path, RULE_CONFIG_BUDGET) else {
        report["reason"] = json!("rule_config_read_failed");
        return;
    };
    // 读取后复核预算与类型，防止读取期间输入变化。
    if bytes.len() as u64 > RULE_CONFIG_BUDGET
        || path
            .symlink_metadata()
            .is_ok_and(|meta| meta.file_type().is_symlink())
    {
        report["reason"] = json!("rule_config_read_failed");
        return;
    }
    report["size_bytes"] = json!(bytes.len());
    report["config_sha256"] = json!(digest(&bytes));
    let Ok(text) = std::str::from_utf8(&bytes) else {
        report["state"] = json!("invalid");
        report["reason"] = json!("rule_config_not_utf8");
        return;
    };
    if text.trim().is_empty() {
        report["state"] = json!("invalid");
        report["reason"] = json!("rule_config_empty");
        return;
    }
    if text
        .bytes()
        .any(|b| b < 0x20 && b != b'\t' && b != b'\n' && b != b'\r')
    {
        report["state"] = json!("invalid");
        report["reason"] = json!("rule_config_control_characters");
        return;
    }
    if text.lines().any(|line| {
        let indent_end = line.len() - line.trim_start_matches([' ', '\t']).len();
        line[..indent_end].contains('\t')
    }) {
        report["state"] = json!("invalid");
        report["reason"] = json!("rule_config_tab_indentation");
        return;
    }
    if !text.lines().any(is_top_level_mapping_key) {
        report["state"] = json!("invalid");
        report["reason"] = json!("rule_config_not_mapping_shaped");
        return;
    }
    report["state"] = json!("configured");
    report["reason"] = json!("rule_config_observed_mapping_shaped");
}

/// 保守 YAML 子集：行首（无缩进）`Key:` 形态；足够证明文档含映射键，
/// 不是完整 YAML 解析，语义恒为 unresolved。行尾 CR 仅按 CRLF 容忍剥离。
fn is_top_level_mapping_key(line: &str) -> bool {
    let line = line.trim_end_matches('\r');
    let mut chars = line.chars();
    match chars.next() {
        Some('A'..='Z' | 'a'..='z' | '_') => {}
        _ => return false,
    }
    let key_body =
        |ch: char| matches!(ch, 'A'..='Z' | 'a'..='z' | '0'..='9' | '_' | '-' | '.' | '/');
    let mut rest = chars.as_str();
    let mut key_len = 0usize;
    while let Some(ch) = rest.chars().next() {
        if !key_body(ch) {
            break;
        }
        key_len += ch.len_utf8();
        rest = &rest[ch.len_utf8()..];
    }
    match rest.strip_prefix(':') {
        Some(after) => key_len > 0 && (after.is_empty() || after.starts_with(' ')),
        None => false,
    }
}

/// Swift 六类别适用性与 SwiftLint 规则配置发现的反例验证（OpenSpec 8.10 路径）。
#[cfg(test)]
mod applicability_tests {
    use super::{applicability_profile, observe_rule_config, rule_config_recheck};
    use codeguard_core::CHECK_CATEGORIES;
    use sha2::Digest;
    use std::{
        path::PathBuf,
        sync::atomic::AtomicBool,
        time::{Duration, Instant},
    };

    fn scratch(tag: &str) -> PathBuf {
        let root = std::env::temp_dir().canonicalize().unwrap().join(format!(
            "codeguard-swift-applicability-{}-{tag}",
            std::process::id()
        ));
        std::fs::remove_dir_all(&root).ok();
        std::fs::create_dir_all(&root).unwrap();
        root
    }

    #[test]
    fn swift_profile_covers_exactly_six_categories_with_evidence() {
        let profile = applicability_profile();
        assert_eq!(profile["language"], "swift");
        assert_eq!(profile["dialect_validation"], "unverified");
        assert_eq!(profile["platform_validation"], "unverified");
        let platforms: Vec<&str> = profile["platforms"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|item| item.as_str())
            .collect();
        assert_eq!(platforms, codeguard_core::CANDIDATE_PLATFORMS.to_vec());
        let slots = profile["categories"].as_array().unwrap();
        assert_eq!(slots.len(), CHECK_CATEGORIES.len());
        for category in CHECK_CATEGORIES {
            let slot = slots
                .iter()
                .find(|slot| slot["category"] == category)
                .unwrap_or_else(|| panic!("缺少类别 {category}"));
            assert_eq!(slot["status"], "gap", "{category} 不得虚报为已验收能力");
            assert_ne!(
                slot["applicability"], "not_applicable",
                "{category} 缺工具不能解释为不适用"
            );
            if category == "build" {
                assert_eq!(slot["applicability"], "project_dependent");
            } else {
                assert_eq!(slot["applicability"], "applicable");
            }
            let tools = slot["tools"].as_array().unwrap();
            assert!(!tools.is_empty(), "{category} 必须登记权威候选工具");
            for tool in tools {
                assert_eq!(tool["version_status"], "unvalidated_project_lock_required");
                assert!(!tool["candidate_argv"].as_array().unwrap().is_empty());
                let sources = tool["sources"].as_array().unwrap();
                assert!(!sources.is_empty(), "{} 缺一手依据", tool["tool_id"]);
                for source in sources {
                    let url = source.as_str().unwrap();
                    assert!(url.starts_with("https://"), "{url} 非一手 https 依据");
                }
            }
            assert!(!slot["coverage_gaps"].as_array().unwrap().is_empty());
        }
        // Swift comments 槽必须声明 DocC 只诊断坏注释、不证明存在性。
        let comments = slots
            .iter()
            .find(|slot| slot["category"] == "comments")
            .unwrap();
        assert!(
            comments["coverage_gaps"]
                .as_array()
                .unwrap()
                .iter()
                .any(|gap| gap
                    .as_str()
                    .unwrap()
                    .contains("docc_diagnoses_malformed_comments_not_presence"))
        );
    }

    #[test]
    fn legal_swiftlint_config_is_configured_without_semantic_or_tool_claims() {
        let root = scratch("legal");
        let body = "disabled_rules:\n  - trailing_whitespace\nline_length: 120\n";
        std::fs::write(root.join(".swiftlint.yml"), body).unwrap();
        let report = observe_rule_config(
            &root,
            Instant::now() + Duration::from_secs(5),
            &AtomicBool::new(false),
        );
        assert_eq!(report["tool_id"], "swiftlint");
        assert_eq!(report["state"], "configured");
        assert_eq!(report["selected"].as_str().unwrap(), ".swiftlint.yml");
        assert_eq!(
            report["config_sha256"].as_str().unwrap(),
            format!("{:x}", sha2::Sha256::digest(body.as_bytes()))
        );
        assert_eq!(report["semantics"], "unresolved");
        assert_eq!(report["tool_execution"], "not_attempted");
        assert_eq!(rule_config_recheck(&root, &report), None);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn absent_config_is_missing_and_fabricates_nothing() {
        let root = scratch("missing");
        let report = observe_rule_config(
            &root,
            Instant::now() + Duration::from_secs(5),
            &AtomicBool::new(false),
        );
        assert_eq!(report["state"], "missing");
        assert_eq!(report["reason"], "rule_config_not_found");
        assert!(report["selected"].is_null());
        assert!(report["config_sha256"].is_null());
        assert!(report["size_bytes"].is_null());
        assert_eq!(report["tool_execution"], "not_attempted");
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn provably_broken_configs_are_invalid() {
        let root = scratch("invalid");
        std::fs::write(
            root.join(".swiftlint.yml"),
            b"disabled_rules:\n\t- force_cast\n",
        )
        .unwrap();
        assert_eq!(
            observe_rule_config(
                &root,
                Instant::now() + Duration::from_secs(5),
                &AtomicBool::new(false),
            )["reason"],
            "rule_config_tab_indentation"
        );

        std::fs::write(root.join(".swiftlint.yml"), b"\xff\xfe not utf8\n").unwrap();
        assert_eq!(
            observe_rule_config(
                &root,
                Instant::now() + Duration::from_secs(5),
                &AtomicBool::new(false),
            )["reason"],
            "rule_config_not_utf8"
        );

        std::fs::write(root.join(".swiftlint.yml"), "\n \n").unwrap();
        assert_eq!(
            observe_rule_config(
                &root,
                Instant::now() + Duration::from_secs(5),
                &AtomicBool::new(false),
            )["reason"],
            "rule_config_empty"
        );

        std::fs::write(
            root.join(".swiftlint.yml"),
            "just a sentence no colon mapping\n",
        )
        .unwrap();
        assert_eq!(
            observe_rule_config(
                &root,
                Instant::now() + Duration::from_secs(5),
                &AtomicBool::new(false),
            )["reason"],
            "rule_config_not_mapping_shaped"
        );
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn yml_and_yaml_coexistence_and_cancellation_stay_unknown() {
        let root = scratch("unknown");
        std::fs::write(root.join(".swiftlint.yml"), "line_length: 120\n").unwrap();
        std::fs::write(root.join(".swiftlint.yaml"), "line_length: 100\n").unwrap();
        let ambiguous = observe_rule_config(
            &root,
            Instant::now() + Duration::from_secs(5),
            &AtomicBool::new(false),
        );
        assert_eq!(ambiguous["state"], "unknown");
        assert_eq!(ambiguous["reason"], "rule_config_candidate_ambiguous");
        assert!(
            ambiguous["found_candidates"]
                .as_array()
                .unwrap()
                .iter()
                .any(|found| found == ".swiftlint.yaml")
        );

        std::fs::remove_file(root.join(".swiftlint.yaml")).unwrap();
        std::fs::remove_file(root.join(".swiftlint.yml")).unwrap();
        #[cfg(unix)]
        {
            std::fs::write(root.join("real-swiftlint.yml"), "line_length: 120\n").unwrap();
            std::os::unix::fs::symlink(
                root.join("real-swiftlint.yml"),
                root.join(".swiftlint.yml"),
            )
            .unwrap();
            let linked = observe_rule_config(
                &root,
                Instant::now() + Duration::from_secs(5),
                &AtomicBool::new(false),
            );
            assert_eq!(linked["state"], "unknown");
            assert_eq!(linked["reason"], "rule_config_symlink_not_followed");
            std::fs::remove_file(root.join(".swiftlint.yml")).unwrap();
        }

        assert_eq!(
            observe_rule_config(
                &root,
                Instant::now() + Duration::from_secs(5),
                &AtomicBool::new(true),
            )["reason"],
            "request_cancelled"
        );
        assert_eq!(
            observe_rule_config(
                &root,
                Instant::now() - Duration::from_millis(1),
                &AtomicBool::new(false),
            )["reason"],
            "request_deadline_exceeded"
        );
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn input_change_invalidates_previous_config_observation() {
        let root = scratch("changed");
        std::fs::write(root.join(".swiftlint.yml"), "line_length: 120\n").unwrap();
        let report = observe_rule_config(
            &root,
            Instant::now() + Duration::from_secs(5),
            &AtomicBool::new(false),
        );
        assert_eq!(report["state"], "configured");
        std::fs::write(root.join(".swiftlint.yml"), "line_length: 100\n").unwrap();
        assert_eq!(
            rule_config_recheck(&root, &report),
            Some("rule_config_changed_after_check")
        );
        std::fs::remove_file(root.join(".swiftlint.yml")).unwrap();
        assert_eq!(
            rule_config_recheck(&root, &report),
            Some("rule_config_removed_after_check")
        );
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn missing_observation_becomes_stale_when_config_appears() {
        let root = scratch("appeared");
        let report = observe_rule_config(
            &root,
            Instant::now() + Duration::from_secs(5),
            &AtomicBool::new(false),
        );
        assert_eq!(report["state"], "missing");
        assert_eq!(
            rule_config_recheck(&root, &report),
            None,
            "候选仍缺时缺配置观察保持当前"
        );
        std::fs::write(root.join(".swiftlint.yml"), "line_length: 120\n").unwrap();
        assert_eq!(
            rule_config_recheck(&root, &report),
            Some("rule_config_now_present_after_check")
        );
        std::fs::remove_dir_all(root).unwrap();
    }
}
