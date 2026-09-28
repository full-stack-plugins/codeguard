//! 误报白名单候选的只读检查命令；项目文件不能自行取得批准。

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use codeguard_core::{
    AllowlistTarget, FalsePositiveIdentity, IdentityMismatch, match_false_positive_identity,
};
use codeguard_runtime::read_bounded_regular_file;
use serde_json::json;
use sha2::{Digest, Sha256};

use crate::false_positive_decision::parse_false_positive_decision_candidate;
use crate::next_command::read_task_brief;
use crate::work_sync::latest_current_finding_observation;

struct Arguments {
    operation: String,
    decision_id: Option<String>,
    paths: Vec<PathBuf>,
    observed_identity: Option<PathBuf>,
    json: bool,
}

/// 读取明确指定的候选文件并展示结构状态；不会签发误报例外。
pub fn run(args: &[String]) -> ExitCode {
    if matches!(args, [first, second, ..] if first == "whitelist" && second == "propose") {
        if args[2..].iter().any(|arg| {
            matches!(
                arg.as_str(),
                "--correct-decision"
                    | "--correction-reason"
                    | "--verification-run"
                    | "--replacement"
            )
        }) {
            return crate::whitelist_correction_command::run(&args[2..]);
        }
        return propose(&args[2..]);
    }
    let parsed = match parse_args(args) {
        Ok(parsed) => parsed,
        Err(message) => {
            eprintln!("{message}");
            return ExitCode::from(2);
        }
    };
    let mut entries = Vec::new();
    let mut candidates = Vec::new();
    let observed = parsed
        .observed_identity
        .as_deref()
        .map(read_observed_identity);
    let mut counts = BTreeMap::<String, usize>::new();
    let mut invalid = 0_usize;
    for path in &parsed.paths {
        match read_candidate(path) {
            Ok(candidate) => {
                *counts.entry(candidate.id.clone()).or_default() += 1;
                let identity = &candidate.identity;
                let (target_kind, target) = match &identity.target {
                    AllowlistTarget::Source { path, .. } => ("source", path.as_str()),
                    AllowlistTarget::Dependency { component, .. } => {
                        ("dependency", component.as_str())
                    }
                };
                entries.push(json!({
                    "id":candidate.id,
                    "finding_id":identity.finding_id,
                    "checker_id":identity.checker_id,
                    "native_rule_id":identity.native_rule_id,
                    "category":identity.category,
                    "target_kind":target_kind,
                    "target":target,
                    "expires_at":candidate.expires_at,
                    "status":"candidate_unverified",
                    "reason":"approval_source_and_time_unverified"
                }));
                candidates.push(candidate);
            }
            Err(_) => {
                invalid += 1;
                entries.push(json!({
                    "id":null,
                    "status":"invalid_candidate",
                    "reason":"unreadable_or_invalid_candidate"
                }));
            }
        }
    }
    let mut conflicts = 0_usize;
    let mut overlapping_ids = BTreeSet::new();
    for (index, first) in candidates.iter().enumerate() {
        for second in candidates.iter().skip(index + 1) {
            // 一个稳定 finding 不能同时指向多个待批目标；查询不能选中其中一条假装无冲突。
            if first.identity.finding_id == second.identity.finding_id {
                overlapping_ids.insert(first.id.as_str());
                overlapping_ids.insert(second.id.as_str());
            }
        }
    }
    for entry in &mut entries {
        let Some(id) = entry["id"].as_str() else {
            continue;
        };
        if counts.get(id).copied().unwrap_or(0) > 1 {
            entry["status"] = json!("conflicting_candidate");
            entry["reason"] = json!("duplicate_decision_id");
            conflicts += 1;
        } else if overlapping_ids.contains(id) {
            entry["status"] = json!("conflicting_candidate");
            entry["reason"] = json!("duplicate_finding_identity");
            conflicts += 1;
        }
    }
    if let Some(id) = &parsed.decision_id {
        entries.retain(|entry| entry["id"] == *id);
    }
    let mut comparison = if let Some(observed) = &observed {
        match observed {
            Err(_) => {
                json!({"status":"invalid_observation","reason":"unreadable_or_invalid_observation"})
            }
            Ok(_) if invalid > 0 => {
                json!({"status":"not_compared","reason":"candidate_set_incomplete"})
            }
            Ok(observed) => {
                let matching: Vec<_> = candidates
                    .iter()
                    .filter(|candidate| {
                        Some(candidate.id.as_str()) == parsed.decision_id.as_deref()
                    })
                    .collect();
                match matching.as_slice() {
                    [candidate] if overlapping_ids.contains(candidate.id.as_str()) => {
                        json!({"status":"not_compared","reason":"duplicate_finding_identity"})
                    }
                    [candidate] => {
                        match match_false_positive_identity(observed, &candidate.identity) {
                            Ok(()) => {
                                json!({"status":"identity_matched","reason":"approval_source_and_time_unverified"})
                            }
                            Err(IdentityMismatch::DifferentIdentity) => {
                                json!({"status":"identity_mismatch","reason":"different_identity",
                                    "mismatch_fields": identity_mismatch_fields(observed, &candidate.identity)})
                            }
                            Err(IdentityMismatch::InvalidObservation) => {
                                json!({"status":"invalid_observation","reason":"invalid_observation_identity"})
                            }
                            Err(IdentityMismatch::InvalidDecision) => {
                                json!({"status":"invalid_candidate","reason":"invalid_candidate_identity"})
                            }
                        }
                    }
                    [] => json!({"status":"not_compared","reason":"decision_id_not_found"}),
                    _ => json!({"status":"not_compared","reason":"duplicate_decision_id"}),
                }
            }
        }
    } else {
        json!({"status":"not_requested","reason":"observed_identity_not_supplied"})
    };
    if comparison.get("mismatch_fields").is_none() {
        comparison["mismatch_fields"] = json!([]);
    }
    comparison["next_action"] = json!(match comparison["status"].as_str() {
        Some("identity_matched") => "verify_independent_approval",
        Some("identity_mismatch") => "rerun_native_checker_and_review_candidate",
        Some("invalid_observation") => "repair_observation_and_rerun_native_checker",
        Some("invalid_candidate") => "repair_candidate",
        _ => "review_candidate_set_and_observation",
    });
    let incomplete = invalid > 0
        || conflicts > 0
        || entries.is_empty()
        || comparison["status"] == "invalid_observation";
    let report = json!({
        "schema_version":"0.3.0",
        "report_type":"whitelist_candidate_inspection",
        "operation":parsed.operation,
        "inspection_status":if incomplete { "incomplete" } else { "complete" },
        "authority":"unverified",
        "gate_effect":"none",
        "invalid_candidate_count":invalid,
        "conflicting_candidate_count":conflicts,
        "comparison":comparison,
        "candidates":entries
    });
    if parsed.json {
        println!("{report}");
    } else {
        println!(
            "误报候选检查：{}；批准来源未核验；门禁不变。",
            report["inspection_status"]
        );
        for entry in report["candidates"].as_array().expect("固定数组") {
            println!(
                "{}：{}（{}）",
                entry["id"].as_str().unwrap_or("invalid"),
                entry["status"].as_str().unwrap_or("invalid_candidate"),
                entry["reason"].as_str().unwrap_or("unknown")
            );
        }
        if parsed.observed_identity.is_some() {
            println!(
                "身份对比：{}（{}）；失配字段：{}；下一步：{}；批准仍未核验。",
                report["comparison"]["status"],
                report["comparison"]["reason"],
                report["comparison"]["mismatch_fields"],
                report["comparison"]["next_action"]
            );
        }
    }
    if incomplete {
        ExitCode::from(3)
    } else {
        ExitCode::SUCCESS
    }
}

fn identity_mismatch_fields(
    observed: &FalsePositiveIdentity,
    decision: &FalsePositiveIdentity,
) -> Vec<&'static str> {
    let mut fields = Vec::new();
    for (different, name) in [
        (observed.finding_id != decision.finding_id, "finding_id"),
        (observed.checker_id != decision.checker_id, "checker_id"),
        (
            observed.native_rule_id != decision.native_rule_id,
            "native_rule_id",
        ),
        (observed.category != decision.category, "category"),
    ] {
        if different {
            fields.push(name);
        }
    }
    match (&observed.target, &decision.target) {
        (
            AllowlistTarget::Source {
                path: left_path,
                file_sha256: left_sha,
            },
            AllowlistTarget::Source {
                path: right_path,
                file_sha256: right_sha,
            },
        ) => {
            if left_path != right_path {
                fields.push("target.path");
            }
            if left_sha != right_sha {
                fields.push("target.file_sha256");
            }
        }
        (
            AllowlistTarget::Dependency {
                component: left_component,
                version: left_version,
                graph_sha256: left_graph,
                advisory_id: left_advisory,
            },
            AllowlistTarget::Dependency {
                component: right_component,
                version: right_version,
                graph_sha256: right_graph,
                advisory_id: right_advisory,
            },
        ) => {
            if left_component != right_component {
                fields.push("target.component");
            }
            if left_version != right_version {
                fields.push("target.version");
            }
            if left_graph != right_graph {
                fields.push("target.graph_sha256");
            }
            if left_advisory != right_advisory {
                fields.push("target.advisory_id");
            }
        }
        _ => fields.push("target.kind"),
    }
    for (different, name) in [
        (
            observed.finding_fingerprint != decision.finding_fingerprint,
            "finding_fingerprint",
        ),
        (observed.tool_sha256 != decision.tool_sha256, "tool_sha256"),
        (
            observed.adapter_sha256 != decision.adapter_sha256,
            "adapter_sha256",
        ),
        (
            observed.rulepack_sha256 != decision.rulepack_sha256,
            "rulepack_sha256",
        ),
    ] {
        if different {
            fields.push(name);
        }
    }
    fields
}

/// 只从本地事实指出误报候选缺少的本轮证据；不制造批准文档。
fn propose(args: &[String]) -> ExitCode {
    let (id, root, json_output, ruff_tool) = match parse_propose_args(args) {
        Ok(parsed) => parsed,
        Err(message) => {
            eprintln!("{message}");
            return ExitCode::from(2);
        }
    };
    let report = match root.canonicalize() {
        Ok(root) if root.is_dir() => match read_task_brief(&root, &id) {
            Ok(brief) if brief["kind"] == "finding" => {
                proposal_for_finding(&root, &id, &brief, ruff_tool.as_deref())
            }
            Ok(_) => proposal_unavailable(&id, "not_a_finding"),
            Err(reason) => proposal_unavailable(&id, reason),
        },
        _ => proposal_unavailable(&id, "project_unreadable"),
    };
    if json_output {
        println!("{report}");
    } else {
        println!(
            "误报白名单候选 {}：{}；缺少完整本轮身份，未生成候选，门禁不变。",
            id, report["status"]
        );
        if matches!(
            report["status"].as_str(),
            Some("evidence_incomplete" | "rulepack_mapping_unavailable")
        ) {
            println!("需补证据：{}", report["missing_evidence"]);
            println!("下一步：{}", report["next_actions"]);
        }
    }
    ExitCode::from(3)
}

fn proposal_for_finding(
    root: &std::path::Path,
    id: &str,
    brief: &serde_json::Value,
    ruff_tool: Option<&Path>,
) -> serde_json::Value {
    let observation = latest_current_finding_observation(root, id).and_then(|observed| {
        if let Some(tool) = ruff_tool {
            let bytes = read_bounded_regular_file(tool, 128 * 1024 * 1024)
                .map_err(|_| "native_tool_unavailable")?;
            let current = format!("{:x}", Sha256::digest(bytes));
            if observed["tool_sha256"] != current {
                return Err("native_tool_changed_since_scan");
            }
        }
        Ok(observed)
    });
    let (status, reason, observed, missing, next_actions): (
        &str,
        &str,
        serde_json::Value,
        Vec<&str>,
        Vec<&str>,
    ) = match observation {
        Ok(observed)
            if observed["native_rule_id"] == brief["native_rule_id"]
                && observed["target"] == brief["scope"] =>
        {
            let mapping_unavailable = observed["rulepack_status"] != "candidate_unapproved"
                || observed["codeguard_rule_id"].is_null()
                || observed["rulepack_sha256"].is_null();
            let mut missing = vec![
                "approved_rulepack_identity",
                "human_false_positive_adjudication",
                "trusted_policy_revision",
            ];
            if mapping_unavailable {
                missing.insert(0, "reviewed_native_rule_mapping");
            }
            if observed["tool_sha256"].is_null() {
                missing.insert(0, "tool_sha256");
            }
            if observed["adapter_sha256"].is_null() {
                missing.insert(0, "adapter_sha256");
            }
            (
                if mapping_unavailable {
                    "rulepack_mapping_unavailable"
                } else {
                    "evidence_incomplete"
                },
                if mapping_unavailable {
                    "native_rule_mapping_or_tool_version_unreviewed"
                } else {
                    "approval_and_approved_rulepack_identity_unavailable"
                },
                observed,
                missing,
                if mapping_unavailable {
                    vec![
                        "run_original_checker_recheck",
                        "review_native_rule_and_tool_version",
                        "request_independent_review",
                    ]
                } else {
                    vec![
                        "run_original_checker_recheck",
                        "capture_complete_current_native_identity",
                        "request_independent_review",
                    ]
                },
            )
        }
        Ok(_) => (
            "latest_report_invalid",
            "task_report_identity_mismatch",
            serde_json::Value::Null,
            vec![
                "current_native_finding",
                "tool_sha256",
                "adapter_sha256",
                "approved_rulepack_identity",
                "human_false_positive_adjudication",
            ],
            vec![
                "run_original_checker_recheck",
                "capture_complete_current_native_identity",
                "request_independent_review",
            ],
        ),
        Err(reason) => (
            reason,
            reason,
            serde_json::Value::Null,
            vec![
                "current_native_finding",
                "tool_sha256",
                "adapter_sha256",
                "approved_rulepack_identity",
                "human_false_positive_adjudication",
            ],
            vec![
                "run_original_checker_recheck",
                "capture_complete_current_native_identity",
                "request_independent_review",
            ],
        ),
    };
    json!({
        "schema_version":"0.4.0", "report_type":"whitelist_proposal_preview",
        "operation":"whitelist_propose", "command_status":"incomplete", "exit_code":3,
        "finding_id":id, "status":status, "reason":reason,
        "checker_id":brief["checker_id"], "native_rule_id":brief["native_rule_id"],
        "target":brief["scope"], "observed_artifacts":observed, "candidate":null,
        "missing_evidence":missing,
        "recheck_argv":brief["recheck_argv"],
        "next_actions":next_actions,
        "authority":"unverified", "gate_effect":"none", "delivery_decision":"not_evaluated"
    })
}

fn proposal_unavailable(id: &str, reason: &str) -> serde_json::Value {
    json!({
        "schema_version":"0.2.0", "report_type":"whitelist_proposal_preview",
        "operation":"whitelist_propose", "command_status":"incomplete", "exit_code":3,
        "finding_id":id, "status":reason, "reason":reason, "candidate":null,
        "missing_evidence":[], "observed_artifacts":null, "recheck_argv":null, "next_actions":[],
        "authority":"unverified", "gate_effect":"none", "delivery_decision":"not_evaluated"
    })
}

fn parse_propose_args(args: &[String]) -> Result<(String, PathBuf, bool, Option<PathBuf>), String> {
    let id = args.first().ok_or("propose 缺少 finding ID")?;
    let suffix = id.strip_prefix("CG-B-").or_else(|| id.strip_prefix("CG-"));
    if !suffix.is_some_and(|part| {
        part.len() == 32
            && part
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
    }) {
        return Err("finding ID 无效".into());
    }
    let mut root = None;
    let mut json_output = false;
    let mut ruff_tool = None;
    let mut index = 1;
    while index < args.len() {
        let arg = &args[index];
        if arg == "--format" {
            index += 1;
            let value = args.get(index).ok_or("--format 缺少值")?;
            json_output = match value.as_str() {
                "json" => true,
                "human" => false,
                _ => return Err("--format 仅支持 human/json".into()),
            };
        } else if let Some(value) = arg.strip_prefix("--format=") {
            json_output = match value {
                "json" => true,
                "human" => false,
                _ => return Err("--format 仅支持 human/json".into()),
            };
        } else if arg == "--ruff-tool" {
            index += 1;
            let value = args.get(index).ok_or("--ruff-tool 缺少值")?;
            let path = PathBuf::from(value);
            if !path.is_absolute() || ruff_tool.replace(path).is_some() {
                return Err("--ruff-tool 需要唯一的绝对路径".into());
            }
        } else if arg.starts_with('-') || root.is_some() {
            return Err(format!("不支持的参数：{arg}"));
        } else {
            root = Some(PathBuf::from(arg));
        }
        index += 1;
    }
    Ok((
        id.clone(),
        root.unwrap_or_else(|| PathBuf::from(".")),
        json_output,
        ruff_tool,
    ))
}

fn read_candidate(
    path: &Path,
) -> Result<crate::false_positive_decision::FalsePositiveDecisionCandidate, String> {
    let bytes = read_bounded_file(path, 128 * 1024).map_err(|_| "候选不可读取")?;
    parse_false_positive_decision_candidate(&bytes)
}

fn read_observed_identity(path: &Path) -> Result<FalsePositiveIdentity, String> {
    let bytes = read_bounded_file(path, 16 * 1024)?;
    serde_json::from_slice(&bytes).map_err(|_| "观察身份格式无效".into())
}

fn read_bounded_file(path: &Path, limit: u64) -> Result<Vec<u8>, String> {
    read_bounded_regular_file(path, limit).map_err(|_| "文件不是有界普通文件".into())
}

fn parse_args(args: &[String]) -> Result<Arguments, String> {
    if args.first().map(String::as_str) != Some("whitelist") {
        return Err("rules 当前仅支持 whitelist".into());
    }
    let operation = args
        .get(1)
        .map(String::as_str)
        .ok_or("缺少 whitelist 子命令")?;
    if !matches!(operation, "list" | "explain") {
        return Err("whitelist 当前仅支持 list/explain".into());
    }
    let mut index = 2;
    let decision_id = if operation == "explain" {
        let id = args.get(index).ok_or("explain 缺少决策 ID")?;
        if id.is_empty() || id.starts_with('-') || id.chars().any(char::is_control) {
            return Err("决策 ID 无效".into());
        }
        index += 1;
        Some(id.clone())
    } else {
        None
    };
    let mut paths = Vec::new();
    let mut observed_identity = None;
    let mut json = false;
    while index < args.len() {
        let arg = &args[index];
        if arg == "--candidate" || arg == "--format" || arg == "--observed-identity" {
            index += 1;
            let value = args.get(index).ok_or_else(|| format!("{arg} 缺少值"))?;
            if arg == "--candidate" {
                paths.push(PathBuf::from(value));
            } else if arg == "--observed-identity" {
                if operation != "explain" || observed_identity.is_some() {
                    return Err("--observed-identity 仅可在 explain 指定一次".into());
                }
                observed_identity = Some(PathBuf::from(value));
            } else {
                json = match value.as_str() {
                    "json" => true,
                    "human" => false,
                    _ => return Err("--format 仅支持 human/json".into()),
                };
            }
        } else if let Some(value) = arg.strip_prefix("--format=") {
            json = match value {
                "json" => true,
                "human" => false,
                _ => return Err("--format 仅支持 human/json".into()),
            };
        } else {
            return Err(format!("不支持的参数：{arg}"));
        }
        index += 1;
    }
    if paths.is_empty() || paths.len() > 100 || paths.iter().any(|path| path.as_os_str().is_empty())
    {
        return Err("须指定 1–100 个 --candidate 文件".into());
    }
    Ok(Arguments {
        operation: operation.into(),
        decision_id,
        paths,
        observed_identity,
        json,
    })
}
