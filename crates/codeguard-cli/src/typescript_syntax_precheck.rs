//! 缺显式 ESLint 上下文时的 TypeScript 单文件候选语法初检；不替代原生义务。

use crate::eslint_lint_arguments::EslintLintArguments;
use crate::syntax_worker_runner::run_syntax_worker_candidate;
use codeguard_adapters::SourceMap;
use codeguard_core::SyntaxRecoveryAnchor;
use codeguard_runtime::read_bounded_regular_file;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::sync::atomic::AtomicBool;
use std::time::Instant;

/// 仅为无任何显式原生上下文的 TypeScript/JavaScript 普通单文件请求启用候选初检。
/// 参数为原命令请求；返回是否可尝试补充语法观察，不断言原生工具已缺失。
pub(crate) fn eligible(args: &EslintLintArguments) -> bool {
    args.node.is_none()
        && args.entry.is_none()
        && args.config.is_none()
        && args.config_map.is_none()
        && args.cwd.is_none()
        && args.version.is_none()
        && std::fs::symlink_metadata(&args.source).is_ok_and(|metadata| metadata.is_file())
        && args.source.extension().is_some_and(|extension| {
            ["ts", "tsx", "mts", "cts", "js", "jsx", "mjs", "cjs"]
                .iter()
                .any(|value| extension == *value)
        })
}

/// 使用本轮源码快照与私有 Rust worker 补充疑似语法观察；原生仍未运行。
/// 参数为受支持的单文件请求及共同截止时间；返回无交付权威的 TypeScript 0.3.0 或 TSX 0.4.0 反馈。
pub(crate) fn observe(args: &EslintLintArguments, deadline: Instant) -> Value {
    let language = if args
        .source
        .extension()
        .is_some_and(|extension| extension == "tsx")
    {
        "tsx"
    } else {
        "typescript"
    };
    let source_name = args
        .source
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("source.ts");
    let mut report = json!({
        "schema_version": if language == "tsx" { "0.4.0" } else { "0.3.0" },
        "report_type": "eslint_local_feedback",
        "status": "incomplete",
        "local_coherent": false,
        "coverage_proven": false,
        "delivery_decision": "not_evaluated",
        "reason": "eslint_execution_context_missing",
        "findings": [],
        "suppressed_count": 0,
        "workbench_status": "not_connected",
        "workbench": null,
        "native": {"status": "not_run", "reason": "explicit_context_missing"},
        "setup": {"requirement": "required", "reason": "native_confirmation_needed", "task_id": null},
        "syntax_precheck": {
            "backend": "bundled_tree_sitter_wasm_candidate",
            "language": language,
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
        "next_action": "提供原生 ESLint/TypeScript 语法确认上下文；候选初检不能代替原生检查"
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
        language,
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
            "先核对疑似语法位置，再用适用的原生 TypeScript/ESLint 能力确认；勿仅凭初检修改源码或关闭任务"
        );
    }
    report["syntax_precheck"]["observations"] = json!(mapped);
    report
}

pub(crate) fn map_observations(
    source: &[u8],
    recoveries: &[crate::syntax_worker_recovery::SyntaxWorkerRecovery],
) -> Result<Vec<Value>, String> {
    let map = SourceMap::new(source)?;
    recoveries
        .iter()
        .map(|recovery| {
            let kind = match recovery.kind.as_str() {
                "ERROR" => "ERROR",
                "MISSING" => "MISSING",
                _ => return Err("syntax_recovery_kind_invalid".into()),
            };
            let anchor = SyntaxRecoveryAnchor {
                kind,
                group_id: recovery.group_id,
                syntax_kind: recovery.syntax_kind.clone(),
                start_byte: recovery.start_byte,
                end_byte: recovery.end_byte,
                start_row: recovery.start_row,
                start_column_byte: recovery.start_column_byte,
                end_row: recovery.end_row,
                end_column_byte: recovery.end_column_byte,
            };
            let position = map.map(&anchor)?;
            Ok(json!({
                "classification": "suspected",
                "kind": kind,
                "syntax_kind": position.syntax_kind,
                "start_line": position.start_line,
                "start_column": position.start_column_scalar,
                "end_line": position.end_line,
                "end_column": position.end_column_scalar,
                "group_id": position.group_id
            }))
        })
        .collect()
}
