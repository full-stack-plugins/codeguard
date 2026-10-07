//! 项目范围内 Ruby 原生优先观察；仅调用冻结单文件 ruby -c。
use crate::{plain_syntax_source::read_plain_source, ruby_tool_selection::RubyToolSelection};
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
    let selection = RubyToolSelection::discover(tool);
    let mut report = json!({"schema_version":"0.1.0","report_type":"ruby_syntax_scan","scope":"single_frozen_ruby_files","tool_selection":selection.report(),"source_file_count":paths.len(),"unobserved_count":paths.len().saturating_sub(64),"scope_stable":true,"local_parse_complete":false,"files":[],"task_status":"not_connected","authority":"local_unverified","coverage_proven":false,"delivery_decision":"not_evaluated"});
    let mut files = Vec::new();
    for relative in paths.iter().take(64) {
        let mut file = json!({"path":relative,"source_sha256":null,"current":false,"tool_path":selection.tool().and_then(|p|p.canonicalize().ok()),"native":crate::ruby_lint_command::unavailable("ruby_tool_not_found"),"task_id":null,"task_sync_reason":null,"recheck_argv":null,"next_action":"读取候选初检；疑似异常需要原生确认，完整零恢复时推荐准备原生工具"});
        if cancelled.load(Ordering::Relaxed) || codeguard_runtime::sigint_cancellation_requested() {
            file["native"] = crate::ruby_lint_command::unavailable("request_cancelled");
            files.push(file);
            continue;
        }
        if Instant::now() >= deadline {
            file["native"] = crate::ruby_lint_command::unavailable("request_deadline_exceeded");
            files.push(file);
            continue;
        }
        let source = match read_plain_source(&root.join(relative)) {
            Ok(s) => s,
            Err(_) => {
                file["native"] =
                    crate::ruby_lint_command::unavailable("ruby_source_unavailable_or_invalid");
                files.push(file);
                continue;
            }
        };
        file["source_sha256"] = json!(digest(&source));
        file["current"] = json!(true);
        if let Some(tool) = selection.tool() {
            file["native"] = crate::ruby_project_version::observe(
                root,
                &root.join(relative),
                tool,
                &source,
                deadline,
                cancelled,
            );
            let has_syntax = file["native"]["diagnostics"]
                .as_array()
                .is_some_and(|r| !r.is_empty());
            file["next_action"] = json!(if has_syntax {
                "先核对项目Ruby版本是否适用，再按当前原生行号确认和修复语法；不猜测列号，继续RuboCop与完整项目检查"
            } else if file["native"]["status"] == "completed" {
                "本文件原生零诊断；继续项目 lint、类型、注释、安全与依赖检查，不凭此关闭任务"
            } else {
                "核对项目声明版本、选定Ruby2.6.10p210及预算后原工具复检；不改无关源码、不换工具绕过失败"
            });
            if file["native"]["version"] == "ruby 2.6.10p210" {
                file["recheck_argv"] = json!([
                    "codeguard",
                    "lint",
                    "ruby",
                    root.join(relative),
                    "--ruby-tool",
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
        } else if matches!(
            file["native"]["status"].as_str(),
            Some("completed" | "diagnostics_observed")
        ) && file["path"]
            .as_str()
            .is_none_or(|p| !crate::ruby_project_version::compatible(root, &root.join(p)))
        {
            Some("ruby_project_version_changed_after_check")
        } else if file["tool_path"]
            .as_str()
            .is_some_and(|p| resolved.as_deref() != Some(Path::new(p)))
            || file["native"]["tool_sha256"]
                .as_str()
                .is_some_and(|s| sha.as_deref() != Some(s))
        {
            Some("ruby_tool_changed_after_check")
        } else if file["path"]
            .as_str()
            .and_then(|p| read_plain_source(&root.join(p)).ok())
            .is_none_or(|b| file["source_sha256"] != digest(&b))
        {
            Some("ruby_source_changed_after_check")
        } else {
            None
        };
        if let Some(reason) = reason {
            file["current"] = json!(false);
            file["native"] = crate::ruby_lint_command::unavailable(reason);
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
        && relative.ends_with(".rb")
        && (report["files"]
            .as_array()
            .is_some_and(|r| r.iter().any(|f| f["path"] == relative))
            || report["unobserved_count"].as_u64().is_some_and(|n| n > 0))
}
fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

/// RuboCop 规则配置的候选位置；工具从项目根向下查找默认配置，两个扩展名均接受。
const RULE_CONFIG_CANDIDATES: [&str; 2] = [".rubocop.yml", ".rubocop.yaml"];
/// 配置文件读取预算；超出即无法完整判定，不猜测内容。
const RULE_CONFIG_BUDGET: u64 = 1024 * 1024;

/// 在 CLI 侧呈现已发布的 Ruby 六类别研究档案（OpenSpec 8.16，
/// rulepacks/ruby_static_candidate_v1.json 经 adapters 严格读取）。
/// 档案本身不运行工具、不授予能力；此处不复制数据，避免双源漂移。
#[cfg_attr(not(test), allow(dead_code))]
pub(crate) fn applicability_profile() -> Value {
    let profile =
        codeguard_adapters::bundled_ruby_candidate_profile().expect("内置 Ruby 候选档案必须可解析");
    json!({
        "schema_version": profile.schema_version,
        "language": profile.language,
        "runtime_dialects": profile.runtime_dialects,
        "runtime_validation": profile.runtime_validation,
        "platforms": profile.platforms,
        "platform_validation": profile.platform_validation,
        "target_version_inputs": profile.target_version_inputs,
        "version_gaps": profile.version_gaps,
        "categories": profile
            .categories
            .iter()
            .map(|slot| {
                json!({
                    "category": slot.category,
                    "applicability": slot.applicability,
                    "status": slot.status,
                    "tools": slot
                        .tools
                        .iter()
                        .map(|tool| {
                            json!({
                                "tool_id": tool.tool_id,
                                "candidate_version": tool.candidate_version,
                                "version_status": tool.version_status,
                                "candidate_argv": tool.candidate_argv,
                                "scope": tool.scope,
                                "sources": tool.sources,
                            })
                        })
                        .collect::<Vec<_>>(),
                    "requires_database_freshness": slot.requires_database_freshness,
                    "coverage_gaps": slot.coverage_gaps,
                })
            })
            .collect::<Vec<_>>(),
    })
}

/// 只读发现项目 RuboCop 规则配置并区分 configured/missing/invalid/unknown；
/// 与其它观察共用截止时间和取消令牌，不运行 RuboCop/Bundler、不安装工具。
/// `configured` 只表示文件存在且结构上是映射形态的 YAML 子集，不证明规则语义或工具可用。
#[cfg_attr(not(test), allow(dead_code))]
pub(crate) fn observe_rule_config(root: &Path, deadline: Instant, cancelled: &AtomicBool) -> Value {
    let mut report = json!({
        "schema_version": "0.1.0",
        "report_type": "ruby_rubocop_rule_config",
        "tool_id": "rubocop",
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

/// 保守 YAML 子集：行首（无缩进）`Key:` 形态（含 RuboCop 的 `Style/Rule:` 部门键）；
/// 足够证明文档含映射键，不是完整 YAML 解析，语义恒为 unresolved。行尾 CR 按 CRLF 容忍剥离。
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

/// Ruby 六类别适用性（8.16 档案在 CLI 侧的呈现）与 RuboCop 规则配置发现反例。
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
            "codeguard-ruby-applicability-{}-{tag}",
            std::process::id()
        ));
        std::fs::remove_dir_all(&root).ok();
        std::fs::create_dir_all(&root).unwrap();
        root
    }

    #[test]
    fn ruby_profile_surfaces_the_registered_six_categories() {
        let profile = applicability_profile();
        assert_eq!(profile["language"], "ruby");
        let slots = profile["categories"].as_array().unwrap();
        assert_eq!(slots.len(), CHECK_CATEGORIES.len());
        for category in CHECK_CATEGORIES {
            let slot = slots
                .iter()
                .find(|slot| slot["category"] == category)
                .unwrap_or_else(|| panic!("缺少类别 {category}"));
            assert_eq!(slot["status"], "gap");
            assert_ne!(slot["applicability"], "not_applicable");
            if category == "build" {
                assert_eq!(slot["applicability"], "project_dependent");
            }
        }
        // 注册表必须声明 bundler-audit 的数据库时效缺口。
        let cve = slots.iter().find(|slot| slot["category"] == "cve").unwrap();
        assert!(
            cve["tools"]
                .as_array()
                .unwrap()
                .iter()
                .any(|tool| tool["tool_id"] == "bundler_audit")
        );
        assert!(cve["requires_database_freshness"] == true);
    }

    #[test]
    fn legal_rubocop_config_is_configured_without_semantic_or_tool_claims() {
        let root = scratch("legal");
        let body = "AllCops:\n  TargetRubyVersion: 3.2\nStyle/Documentation:\n  Enabled: true\n";
        std::fs::write(root.join(".rubocop.yml"), body).unwrap();
        let report = observe_rule_config(
            &root,
            Instant::now() + Duration::from_secs(5),
            &AtomicBool::new(false),
        );
        assert_eq!(report["tool_id"], "rubocop");
        assert_eq!(report["state"], "configured");
        assert_eq!(report["selected"].as_str().unwrap(), ".rubocop.yml");
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
        assert_eq!(report["tool_execution"], "not_attempted");
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn provably_broken_configs_are_invalid() {
        let root = scratch("invalid");
        std::fs::write(
            root.join(".rubocop.yml"),
            b"AllCops:\n\tTargetRubyVersion: 3.2\n",
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

        std::fs::write(root.join(".rubocop.yml"), b"\xff\xfe\n").unwrap();
        assert_eq!(
            observe_rule_config(
                &root,
                Instant::now() + Duration::from_secs(5),
                &AtomicBool::new(false),
            )["reason"],
            "rule_config_not_utf8"
        );

        std::fs::write(root.join(".rubocop.yml"), "  \n").unwrap();
        assert_eq!(
            observe_rule_config(
                &root,
                Instant::now() + Duration::from_secs(5),
                &AtomicBool::new(false),
            )["reason"],
            "rule_config_empty"
        );

        std::fs::write(root.join(".rubocop.yml"), "plain sentence\n").unwrap();
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
    fn ambiguous_cancellation_and_symlink_stay_unknown() {
        let root = scratch("unknown");
        std::fs::write(root.join(".rubocop.yml"), "AllCops:\n  NewCops: enable\n").unwrap();
        std::fs::write(root.join(".rubocop.yaml"), "AllCops:\n  NewCops: disable\n").unwrap();
        let ambiguous = observe_rule_config(
            &root,
            Instant::now() + Duration::from_secs(5),
            &AtomicBool::new(false),
        );
        assert_eq!(ambiguous["state"], "unknown");
        assert_eq!(ambiguous["reason"], "rule_config_candidate_ambiguous");

        std::fs::remove_file(root.join(".rubocop.yaml")).unwrap();
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
        #[cfg(unix)]
        {
            std::fs::remove_file(root.join(".rubocop.yml")).unwrap();
            std::fs::write(
                root.join("real-rubocop.yml"),
                "AllCops:\n  NewCops: enable\n",
            )
            .unwrap();
            std::os::unix::fs::symlink(root.join("real-rubocop.yml"), root.join(".rubocop.yml"))
                .unwrap();
            let linked = observe_rule_config(
                &root,
                Instant::now() + Duration::from_secs(5),
                &AtomicBool::new(false),
            );
            assert_eq!(linked["state"], "unknown");
            assert_eq!(linked["reason"], "rule_config_symlink_not_followed");
        }
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn input_change_invalidates_previous_config_observation() {
        let root = scratch("changed");
        std::fs::write(root.join(".rubocop.yml"), "AllCops:\n  NewCops: enable\n").unwrap();
        let report = observe_rule_config(
            &root,
            Instant::now() + Duration::from_secs(5),
            &AtomicBool::new(false),
        );
        assert_eq!(report["state"], "configured");
        std::fs::write(root.join(".rubocop.yml"), "AllCops:\n  NewCops: disable\n").unwrap();
        assert_eq!(
            rule_config_recheck(&root, &report),
            Some("rule_config_changed_after_check")
        );
        std::fs::remove_file(root.join(".rubocop.yml")).unwrap();
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
        std::fs::write(root.join(".rubocop.yml"), "AllCops:\n  NewCops: enable\n").unwrap();
        assert_eq!(
            rule_config_recheck(&root, &report),
            Some("rule_config_now_present_after_check")
        );
        std::fs::remove_dir_all(root).unwrap();
    }
}
