//! 显式单文件候选语法观察；不将未经验收的 grammar 结果变为 lint 结论。

use crate::syntax_worker_runner::run_syntax_worker_candidate;
use codeguard_adapters::bundled_grammar_candidate;
use serde_json::json;
use std::fs::File;
use std::io::Read;
use std::path::{Component, Path, PathBuf};
use std::process::ExitCode;
use std::sync::atomic::AtomicBool;
use std::time::{Duration, Instant};

/// 对固定语言和普通 UTF-8 文件执行隔离候选解析，始终返回未完成。
/// 参数依次是语言、源码文件和可选 `--format=json`；返回 3 表示需要原生确认。
pub fn run(args: &[String]) -> ExitCode {
    let (language, source_path) = match args {
        [language, source] => (language, source),
        [language, source, format] if format == "--format=json" => (language, source),
        _ => {
            eprintln!("用法: grammar probe <language> <file> [--format=json]");
            return ExitCode::from(2);
        }
    };
    if bundled_grammar_candidate(language).is_err() {
        eprintln!("不支持或无法核验的 grammar 语种");
        return ExitCode::from(3);
    }
    let path = PathBuf::from(source_path);
    let source = match read_plain_source(&path) {
        Ok(source) => source,
        Err(reason) => {
            eprintln!("语法候选观察未完成：{reason}");
            return ExitCode::from(3);
        }
    };
    let executable = match std::env::current_exe() {
        Ok(path) => path,
        Err(_) => return ExitCode::from(4),
    };
    let deadline = Instant::now() + Duration::from_secs(90);
    let relative_name = match path.file_name().and_then(|name| name.to_str()) {
        Some(name) => name,
        None => return ExitCode::from(3),
    };
    let result = run_syntax_worker_candidate(
        &executable,
        language,
        relative_name,
        &source,
        deadline,
        &AtomicBool::new(false),
    );
    match result {
        Ok(observation) => {
            println!(
                "{}",
                json!({
                    "schema_version":"0.1.0",
                    "report_type":"grammar_candidate_probe",
                    "language":language,
                    "path":source_path,
                    "source_sha256":observation.source_sha256,
                    "grammar_sha256":observation.grammar_sha256,
                    "grammar_qualified":false,
                    "precheck":observation.precheck,
                    "recoveries":observation.recoveries,
                    "native":{"status":"not_run","reason":"explicit_candidate_probe"},
                    "delivery_decision":"not_evaluated",
                    "next_action":if observation.recoveries.is_empty() {
                        "run_or_configure_applicable_native_lint_before_delivery"
                    } else {
                        "confirm_suspected_recoveries_with_applicable_native_tool"
                    }
                })
            );
            ExitCode::from(3)
        }
        Err(reason) => {
            println!(
                "{}",
                json!({
                    "schema_version":"0.1.0",
                    "report_type":"grammar_candidate_probe",
                    "language":language,
                    "path":source_path,
                    "status":"incomplete",
                    "reason":reason,
                    "native":{"status":"not_run","reason":"explicit_candidate_probe"},
                    "delivery_decision":"not_evaluated"
                })
            );
            ExitCode::from(3)
        }
    }
}

fn read_plain_source(path: &Path) -> Result<Vec<u8>, &'static str> {
    if path
        .components()
        .any(|component| matches!(component, Component::ParentDir))
    {
        return Err("source_path_parent_component_disallowed");
    }
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .map_err(|_| "source_cwd_unavailable")?
            .join(path)
    };
    let mut prefix = PathBuf::new();
    for component in absolute.components() {
        prefix.push(component);
        let metadata = std::fs::symlink_metadata(&prefix).map_err(|_| "source_path_unavailable")?;
        if metadata.file_type().is_symlink() {
            return Err("source_path_symlink_disallowed");
        }
    }
    let metadata = std::fs::metadata(&absolute).map_err(|_| "source_unavailable")?;
    if !metadata.is_file() || metadata.len() > 1024 * 1024 {
        return Err("source_not_regular_or_too_large");
    }
    let mut source = Vec::new();
    File::open(&absolute)
        .map_err(|_| "source_unreadable")?
        .take(1024 * 1024 + 1)
        .read_to_end(&mut source)
        .map_err(|_| "source_unreadable")?;
    if source.len() > 1024 * 1024 || std::str::from_utf8(&source).is_err() {
        return Err("source_size_or_encoding_invalid");
    }
    Ok(source)
}
