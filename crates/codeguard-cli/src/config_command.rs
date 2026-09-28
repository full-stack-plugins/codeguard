//! 项目配置的只读静态观察；本地配置和白名单引用均不能自行取得策略权威。

use crate::quality_policy_candidate::parse_quality_policy_candidate;
use crate::tool_lock::parse_tool_lock_document;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::{SystemTime, UNIX_EPOCH};

const MAX_CONFIG_BYTES: u64 = 128 * 1024;

struct Args {
    operation: String,
    root: PathBuf,
    policy_candidate: Option<PathBuf>,
    json: bool,
}

/// 静态校验或解释本地配置形状；批准来源未接入时固定为未完成。
pub fn run(args: &[String]) -> ExitCode {
    let args = match parse_args(args) {
        Ok(args) => args,
        Err(error) => {
            eprintln!("{error}");
            return ExitCode::from(2);
        }
    };
    let report = inspect(&args);
    if args.json {
        println!("{report}");
    } else {
        println!(
            "配置{}：未完成",
            if args.operation == "validate" {
                "校验"
            } else {
                "解释"
            }
        );
        println!("旧项目配置：{}", report["legacy_config"]["status"]);
        println!("工具锁：{}", report["tool_lock"]["status"]);
        println!(
            "质量策略：{}；本地排除、命令和白名单引用不能自行放行。",
            report["quality_policy"]["status"]
        );
        for diagnostic in report["diagnostics"].as_array().into_iter().flatten() {
            println!("待处理：{}", diagnostic.as_str().unwrap_or("unknown"));
        }
    }
    ExitCode::from(3)
}

fn parse_args(args: &[String]) -> Result<Args, String> {
    let Some(operation) = args.first() else {
        return Err("config 缺少 validate 或 explain".into());
    };
    if !matches!(operation.as_str(), "validate" | "explain") {
        return Err("config 只支持 validate 或 explain".into());
    }
    let mut root = None;
    let mut policy_candidate = None;
    let mut json = false;
    let mut index = 1;
    while index < args.len() {
        let current = &args[index];
        let format = if current == "--format" {
            index += 1;
            Some(args.get(index).ok_or("--format 缺少值")?.as_str())
        } else {
            current.strip_prefix("--format=")
        };
        if let Some(format) = format {
            if !matches!(format, "human" | "json") {
                return Err(format!("不支持的格式：{format}"));
            }
            json = format == "json";
        } else if current == "--policy-candidate" {
            index += 1;
            let path = args.get(index).ok_or("--policy-candidate 缺少路径")?;
            if path.starts_with('-') || policy_candidate.is_some() {
                return Err("--policy-candidate 路径非法或重复".into());
            }
            policy_candidate = Some(PathBuf::from(path));
        } else if let Some(path) = current.strip_prefix("--policy-candidate=") {
            if path.is_empty() || policy_candidate.is_some() {
                return Err("--policy-candidate 路径非法或重复".into());
            }
            policy_candidate = Some(PathBuf::from(path));
        } else if current.starts_with('-') || root.is_some() {
            return Err(format!("不支持的参数：{current}"));
        } else {
            root = Some(PathBuf::from(current));
        }
        index += 1;
    }
    Ok(Args {
        operation: operation.clone(),
        root: root.unwrap_or_else(|| PathBuf::from(".")),
        policy_candidate,
        json,
    })
}

fn inspect(args: &Args) -> Value {
    let mut diagnostics = Vec::new();
    if !args.root.is_dir() {
        diagnostics.push("project_root_unavailable");
    }
    let legacy_config = match read_local_file(&args.root.join("codeguard.json")) {
        Ok(None) => {
            diagnostics.push("legacy_config_missing_or_migration_needed");
            json!({"status":"missing","exclusion_count":0,"custom_command_count":0,"extension_count":0,"gate_scope":null})
        }
        Ok(Some(bytes)) => match inspect_legacy_config(&bytes) {
            Ok(config) => {
                if config["exclusion_count"].as_u64().unwrap_or(0) > 0 {
                    diagnostics.push("legacy_exclusions_require_approved_policy");
                }
                if config["custom_command_count"].as_u64().unwrap_or(0) > 0 {
                    diagnostics.push("legacy_commands_require_tool_lock_and_review");
                }
                if config["gate_scope"] == "delta" {
                    diagnostics.push("legacy_delta_scope_requires_approved_policy");
                }
                diagnostics.push("legacy_config_requires_explicit_migration");
                config
            }
            Err(code) => {
                diagnostics.push(code);
                json!({"status":"invalid","exclusion_count":0,"custom_command_count":0,"extension_count":0,"gate_scope":null})
            }
        },
        Err(code) => {
            diagnostics.push(code);
            json!({"status":"invalid","exclusion_count":0,"custom_command_count":0,"extension_count":0,"gate_scope":null})
        }
    };
    let mut tool_lock_digest = None;
    let tool_lock = match read_local_file(&args.root.join("codeguard.lock.json")) {
        Ok(None) => {
            diagnostics.push("tool_lock_missing");
            json!({"status":"missing","tool_count":0})
        }
        Ok(Some(bytes)) => match parse_tool_lock_document(&bytes) {
            Ok(lock) => {
                tool_lock_digest = Some(format!("{:x}", Sha256::digest(&bytes)));
                json!({"status":"structurally_valid_untrusted","tool_count":lock.tools.len()})
            }
            Err(_) => {
                diagnostics.push("tool_lock_invalid");
                json!({"status":"invalid","tool_count":0})
            }
        },
        Err(code) => {
            diagnostics.push(code);
            json!({"status":"invalid","tool_count":0})
        }
    };
    let quality_policy = match args.policy_candidate.as_ref() {
        None => {
            diagnostics.push("approved_quality_policy_unbound");
            json!({"status":"unbound","required_check_count":0,"source_exclusion_count":0,"candidate_sha256":null,"tool_lock_reference":"not_checked"})
        }
        Some(path) => match read_local_file(path) {
            Ok(Some(bytes)) => match parse_quality_policy_candidate(&bytes) {
                Ok(candidate) => {
                    let reference = match tool_lock_digest.as_deref() {
                        Some(actual) if actual == candidate.tool_lock_sha256 => "matched_untrusted",
                        Some(_) => {
                            diagnostics.push("candidate_tool_lock_mismatch");
                            "mismatch"
                        }
                        None => {
                            diagnostics.push("candidate_tool_lock_unresolved");
                            "unresolved"
                        }
                    };
                    diagnostics.push("quality_policy_candidate_approval_unverified");
                    json!({"status":"candidate_unverified","required_check_count":candidate.required_checks.len(),"source_exclusion_count":candidate.source_exclusions.len(),"candidate_sha256":candidate.sha256,"tool_lock_reference":reference})
                }
                Err(code) => {
                    diagnostics.push(code);
                    json!({"status":"invalid","required_check_count":0,"source_exclusion_count":0,"candidate_sha256":null,"tool_lock_reference":"not_checked"})
                }
            },
            Ok(None) => {
                diagnostics.push("quality_policy_candidate_missing");
                json!({"status":"missing","required_check_count":0,"source_exclusion_count":0,"candidate_sha256":null,"tool_lock_reference":"not_checked"})
            }
            Err(code) => {
                diagnostics.push(code);
                json!({"status":"invalid","required_check_count":0,"source_exclusion_count":0,"candidate_sha256":null,"tool_lock_reference":"not_checked"})
            }
        },
    };
    diagnostics.push("whitelist_authority_unverified");
    let request_id = format!(
        "config-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |duration| duration.as_nanos())
    );
    let warnings = if args.policy_candidate.is_some() {
        vec!["policy_candidate_is_not_approval"]
    } else {
        vec!["approved_quality_policy_unbound"]
    };
    json!({
        "schema_version":"0.2.0",
        "report_type":"config_inspection",
        "operation":args.operation,
        "request_id":request_id,
        "inspection_status":"incomplete",
        "command_status":"incomplete",
        "exit_code":3,
        "authority":"unverified",
        "gate_effect":"none",
        "quality_decision":"not_evaluated",
        "legacy_config":legacy_config,
        "tool_lock":tool_lock,
        "quality_policy":quality_policy,
        "whitelist":{"status":"authority_unverified","candidate_effect":"none"},
        "effective_policy":null,
        "diagnostics":diagnostics,
        "warnings":warnings,
        "next_actions":["migrate_legacy_configuration_explicitly","bind_protected_quality_policy_and_tool_lock","review_exact_whitelist_candidates_outside_project_writable_state"]
    })
}

fn read_local_file(path: &Path) -> Result<Option<Vec<u8>>, &'static str> {
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(_) => return Err("config_metadata_unavailable"),
    };
    if !metadata.file_type().is_file() || metadata.len() > MAX_CONFIG_BYTES {
        return Err("config_not_bounded_regular_file");
    }
    fs::read(path).map(Some).map_err(|_| "config_read_failed")
}

fn inspect_legacy_config(bytes: &[u8]) -> Result<Value, &'static str> {
    let value: Value = serde_json::from_slice(bytes).map_err(|_| "legacy_config_invalid_json")?;
    let root = value.as_object().ok_or("legacy_config_not_object")?;
    for key in root.keys() {
        if !["extensions", "exclude", "gate_scope", "java"].contains(&key.as_str()) {
            return Err("legacy_config_unknown_field");
        }
    }
    let extensions = match root.get("extensions") {
        None => 0,
        Some(value) => {
            let entries = value.as_object().ok_or("legacy_extensions_invalid")?;
            if entries.iter().any(|(extension, language)| {
                extension.len() < 2
                    || !extension.starts_with('.')
                    || !extension[1..]
                        .chars()
                        .all(|c| c.is_ascii_alphanumeric() || c == '_')
                    || language.as_str().is_none_or(str::is_empty)
            }) {
                return Err("legacy_extensions_invalid");
            }
            entries.len()
        }
    };
    let exclusions = match root.get("exclude") {
        None => 0,
        Some(value) => {
            let entries = value.as_array().ok_or("legacy_exclude_invalid")?;
            if entries
                .iter()
                .any(|value| value.as_str().is_none_or(str::is_empty))
            {
                return Err("legacy_exclude_invalid");
            }
            entries.len()
        }
    };
    let gate_scope = root
        .get("gate_scope")
        .map(|value| {
            value
                .as_str()
                .filter(|scope| matches!(*scope, "delta" | "repo"))
                .ok_or("legacy_gate_scope_invalid")
        })
        .transpose()?;
    let commands = match root.get("java") {
        None => 0,
        Some(value) => {
            let java = value.as_object().ok_or("legacy_java_invalid")?;
            if java.keys().any(|key| key != "commands") {
                return Err("legacy_java_unknown_field");
            }
            let Some(commands) = java.get("commands") else {
                return Err("legacy_java_invalid");
            };
            let commands = commands.as_array().ok_or("legacy_java_commands_invalid")?;
            if commands.iter().any(|command| {
                command.as_array().is_none_or(|argv| {
                    argv.is_empty()
                        || argv
                            .iter()
                            .any(|arg| arg.as_str().is_none_or(str::is_empty))
                })
            }) {
                return Err("legacy_java_commands_invalid");
            }
            commands.len()
        }
    };
    Ok(json!({
        "status":"structurally_valid_untrusted",
        "extension_count":extensions,
        "exclusion_count":exclusions,
        "custom_command_count":commands,
        "gate_scope":gate_scope,
    }))
}
