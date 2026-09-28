//! 无显式 Maven/P3C 上下文时的 Java 单文件候选语法观察；不替代原生义务。

use crate::java_p3c_command::Args;
use crate::syntax_worker_runner::run_syntax_worker_candidate;
use crate::typescript_syntax_precheck::map_observations;
use codeguard_runtime::read_bounded_regular_file;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::sync::atomic::AtomicBool;
use std::time::Instant;

/// 仅无原生参数且目标是普通 Java 单文件时启用候选解析。
/// 参数为已经通过原生入口解析的请求；返回是否有资格补充非权威语法观察。
pub(crate) fn eligible(args: &Args) -> bool {
    args.maven_tool.is_none()
        && args.java_home.is_none()
        && args.maven_repo.is_none()
        && args.repo_sha256.is_none()
        && std::fs::symlink_metadata(&args.source).is_ok_and(|metadata| metadata.is_file())
        && args
            .source
            .extension()
            .is_some_and(|extension| extension == "java")
}

/// 运行有界私有 worker，返回疑似观察及明确未执行的原生状态。
/// 参数为单文件请求和绝对截止时间；返回报告不具交付放行权威。
pub(crate) fn observe(args: &Args, deadline: Instant) -> Value {
    let source_name = args
        .source
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("source.java");
    let mut report = json!({
        "schema_version": "0.1.0",
        "report_type": "java_syntax_precheck_feedback",
        "status": "incomplete",
        "coverage_proven": false,
        "delivery_decision": "not_evaluated",
        "reason": "native_context_missing",
        "findings": [],
        "native": {"status": "not_run", "reason": "explicit_context_missing"},
        "setup": {"requirement": "required", "reason": "native_confirmation_needed", "task_id": null},
        "syntax_precheck": {
            "backend": "bundled_tree_sitter_wasm_candidate",
            "language": "java",
            "status": "incomplete",
            "reason": "not_run",
            "source_path": source_name,
            "source_sha256": null,
            "grammar_sha256": null,
            "grammar_qualified": false,
            "selected_files": 1,
            "checked_files": 0,
            "truncated": false,
            "observations": []
        },
        "next_action": "提供适用的原生 Java 检查上下文；候选语法初检不能代替 P3C、Javac 或项目构建"
    });
    let source = match read_bounded_regular_file(&args.source, 1024 * 1024) {
        Ok(source) => source,
        Err(_) => {
            report["syntax_precheck"]["reason"] = json!("source_unavailable");
            return report;
        }
    };
    report["syntax_precheck"]["source_sha256"] = json!(format!("{:x}", Sha256::digest(&source)));
    let executable = match std::env::current_exe() {
        Ok(executable) => executable,
        Err(_) => {
            report["syntax_precheck"]["reason"] = json!("worker_executable_unavailable");
            return report;
        }
    };
    let observation = match run_syntax_worker_candidate(
        &executable,
        "java",
        source_name,
        &source,
        deadline,
        &AtomicBool::new(false),
    ) {
        Ok(observation) => observation,
        Err(_) => {
            report["syntax_precheck"]["reason"] = json!("worker_incomplete");
            return report;
        }
    };
    report["syntax_precheck"]["grammar_sha256"] = json!(observation.grammar_sha256);
    report["syntax_precheck"]["checked_files"] = json!(observation.precheck.checked_files);
    report["syntax_precheck"]["truncated"] = json!(observation.precheck.truncated_files > 0);
    report["syntax_precheck"]["reason"] = json!(if observation.precheck.truncated_files > 0 {
        "recovery_budget_truncated"
    } else {
        "grammar_version_unqualified"
    });
    let mapped = match map_observations(&source, &observation.recoveries) {
        Ok(mapped) => mapped,
        Err(_) => {
            report["syntax_precheck"]["reason"] = json!("position_mapping_incomplete");
            report["syntax_precheck"]["checked_files"] = json!(0);
            return report;
        }
    };
    if !mapped.is_empty() {
        report["next_action"] = json!(
            "先核对疑似位置，再用匹配当前 Java 版本的原生语法能力确认；勿仅凭初检改源码或关闭任务"
        );
    }
    report["syntax_precheck"]["observations"] = json!(mapped);
    report
}

/// 向智能体输出有界的疑似位置与原生复检步骤。
/// 参数为本次候选报告；此输出不表示任何检查门禁已通过。
pub(crate) fn print_feedback(report: &Value) {
    let precheck = &report["syntax_precheck"];
    println!("Java 内置语法初检：未完成；原生检查未运行，交付门禁未判定。");
    println!(
        "范围：{}；已检查 {} / {}；疑似语法观察 {} 处。",
        precheck["source_path"].as_str().unwrap_or("source.java"),
        precheck["checked_files"].as_u64().unwrap_or(0),
        precheck["selected_files"].as_u64().unwrap_or(0),
        precheck["observations"].as_array().map_or(0, Vec::len)
    );
    for observation in precheck["observations"]
        .as_array()
        .into_iter()
        .flatten()
        .take(10)
    {
        println!(
            "疑似语法恢复：行 {}，列 {}，类型 {}；需原生确认。",
            observation["start_line"].as_u64().unwrap_or(0),
            observation["start_column"].as_u64().unwrap_or(0),
            observation["kind"].as_str().unwrap_or("unknown")
        );
    }
    println!(
        "下一步：{}",
        report["next_action"]
            .as_str()
            .unwrap_or("提供原生检查上下文")
    );
}
