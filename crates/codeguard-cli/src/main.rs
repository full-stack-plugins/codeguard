//! Codeguard 命令行入口；仅暴露已实现的操作，未完成的检查不宣称门禁通过。

use codeguard_adapters::{LegacyRegistry, capability_row, legacy_registry};
use codeguard_cli::discovery::discover;
use codeguard_cli::tool_identity::current_platform_id;
use codeguard_core::{CANDIDATE_PLATFORMS, CHECK_CATEGORIES};
use codeguard_runtime::NativeObservation;
use std::path::PathBuf;
use std::process::ExitCode;

fn main() -> ExitCode {
    #[cfg(unix)]
    if let Err(error) = codeguard_runtime::install_sigint_cancellation() {
        eprintln!("无法登记 Ctrl-C 取消：{error}");
        return ExitCode::from(4);
    }
    let args: Vec<String> = std::env::args().skip(1).collect();
    if let Some(selection) = codeguard_cli::help_command::trailing_selection(&args) {
        return codeguard_cli::help_command::run(&selection);
    }
    match args.as_slice() {
        [version, rest @ ..] if version == "--version" || version == "-V" => version_report(rest),
        [help, rest @ ..] if matches!(help.as_str(), "--help" | "-h" | "help") => {
            codeguard_cli::help_command::run(rest)
        }
        #[cfg(unix)]
        [command, rest @ ..] if command == "guard-project" => {
            codeguard_cli::guard_integration::command::run(rest)
        }
        [command, rest @ ..] if command == "capabilities" => capabilities(rest),
        [command, rest @ ..] if command == "detect" => detect(rest),
        [command, rest @ ..] if command == "init" => codeguard_cli::init_command::run(rest),
        [command, rest @ ..] if command == "config" => codeguard_cli::config_command::run(rest),
        [command, rest @ ..] if command == "tools" => codeguard_cli::tools_command::run(rest),
        [command, operation, rest @ ..] if command == "grammar" && operation == "status" => {
            codeguard_cli::grammar_status_command::run(rest)
        }
        #[cfg(feature = "wasm-precheck")]
        [command, operation, rest @ ..] if command == "grammar" && operation == "probe" => {
            codeguard_cli::grammar_probe_command::run(rest)
        }
        #[cfg(unix)]
        [command, rest @ ..] if command == "doctor" => codeguard_cli::doctor_command::run(rest),
        [command, rest @ ..] if command == "plan" => codeguard_cli::plan_command::run(rest),
        [command, operation, rest @ ..] if command == "hook" && operation == "plan" => {
            codeguard_cli::hook_plan_command::run(rest)
        }
        #[cfg(unix)]
        [command, operation, rest @ ..] if command == "hook" && operation == "execute" => {
            codeguard_cli::hook_execute_command::run(rest)
        }
        #[cfg(unix)]
        [command, operation, rest @ ..] if command == "hook" && operation == "claude" => {
            codeguard_cli::claude_hook_command::run(rest)
        }
        #[cfg(feature = "wasm-precheck")]
        [command, rest @ ..] if command == "__syntax-worker" => {
            codeguard_cli::syntax_worker_command::run(rest)
        }
        #[cfg(unix)]
        [command, language, rest @ ..] if command == "comments" && language == "rust" => {
            codeguard_cli::rust_comments_command::run(rest)
        }
        #[cfg(unix)]
        [command, language, rest @ ..] if command == "build" && language == "rust" => {
            codeguard_cli::rust_build_command::run(rest)
        }
        #[cfg(unix)]
        [command, rest @ ..] if command == "check" => codeguard_cli::check_command::run(rest),
        #[cfg(unix)]
        [command, language, rest @ ..] if command == "cve" && language == "rust" => {
            codeguard_cli::cargo_audit_command::run(rest)
        }
        #[cfg(unix)]
        [command, language, rest @ ..] if command == "cve" && language == "python" => {
            codeguard_cli::python_cve_command::run(rest)
        }
        #[cfg(unix)]
        [command, rest @ ..] if command == "cve" => codeguard_cli::npm_audit_command::run(rest),
        [command, operation, rest @ ..] if command == "rules" && operation == "list" => {
            codeguard_cli::rules_list_command::run(rest)
        }
        [command, rest @ ..] if command == "rules" => codeguard_cli::whitelist_command::run(rest),
        [command, rest @ ..] if command == "work" => codeguard_cli::work_sync::run(rest),
        [command, rest @ ..] if command == "next" => codeguard_cli::next_command::run(rest),
        [command, rest @ ..] if command == "status" => {
            codeguard_cli::workspace_view_command::run_status(rest)
        }
        [command, rest @ ..]
            if command == "task" && rest.first().is_some_and(|part| part == "show") =>
        {
            codeguard_cli::workspace_view_command::run_show(&rest[1..])
        }
        #[cfg(unix)]
        [command, rest @ ..]
            if command == "task" && rest.first().is_some_and(|part| part == "attempt") =>
        {
            codeguard_cli::task_attempt_command::run(rest)
        }
        #[cfg(unix)]
        [command, rest @ ..]
            if command == "task"
                && matches!(
                    rest.first().map(String::as_str),
                    Some("claim" | "heartbeat" | "release")
                ) =>
        {
            codeguard_cli::task_lease_command::run(rest)
        }
        [command, rest @ ..] if command == "task" => codeguard_cli::task_verify_command::run(rest),
        #[cfg(unix)]
        [command, rest @ ..] if command == "lint" => {
            if rest.first().is_some_and(|language| language == "java") {
                codeguard_cli::java_lint_dispatch::run(&rest[1..])
            } else if rest.first().is_some_and(|language| language == "zig") {
                codeguard_cli::zig_lint_command::run(&rest[1..])
            } else if rest.first().is_some_and(|language| language == "swift") {
                codeguard_cli::swift_lint_command::run(&rest[1..])
            } else if rest.first().is_some_and(|language| language == "kotlin") {
                codeguard_cli::kotlin_lint_command::run(&rest[1..])
            } else if rest.first().is_some_and(|language| language == "erlang") {
                codeguard_cli::erlang_lint_command::run(&rest[1..])
            } else if rest
                .first()
                .is_some_and(|language| language == "typescript")
            {
                codeguard_cli::eslint_lint_command::run(&rest[1..])
            } else if rest.first().is_some_and(|language| language == "go") {
                codeguard_cli::go_lint_command::run(&rest[1..])
            } else if rest.first().is_some_and(|language| language == "ruby") {
                codeguard_cli::ruby_lint_command::run(&rest[1..])
            } else if rest.first().is_some_and(|language| language == "rust") {
                codeguard_cli::rust_lint_command::run(&rest[1..])
            } else if rest.first().is_some_and(|language| language == "shell") {
                codeguard_cli::shell_lint_command::run(&rest[1..])
            } else {
                codeguard_cli::python_lint_command::run(rest)
            }
        }
        #[cfg(unix)]
        [command, rest @ ..] if command == "gate" => {
            codeguard_cli::git_index_safety_command::run(rest)
        }
        _ => {
            eprintln!("未知或尚未实现的命令。使用 codeguard --help 查看可用操作。");
            ExitCode::from(2)
        }
    }
}

fn version_report(args: &[String]) -> ExitCode {
    let format = match args {
        [] => "human",
        [option, value] if option == "--format" && matches!(value.as_str(), "human" | "json") => {
            value.as_str()
        }
        [option] if option == "--format=human" => "human",
        [option] if option == "--format=json" => "json",
        _ => {
            eprintln!("--version 仅支持 --format human|json");
            return ExitCode::from(2);
        }
    };
    let target = current_platform_id()
        .map(str::to_owned)
        .unwrap_or_else(|| format!("{}-{}", std::env::consts::OS, std::env::consts::ARCH));
    let build_identity = option_env!("CODEGUARD_BUILD_SHA").filter(|sha| {
        matches!(sha.len(), 40 | 64) && sha.bytes().all(|byte| byte.is_ascii_hexdigit())
    });
    if format == "json" {
        println!(
            "{}",
            serde_json::json!({
                "schema_version":"0.1.0",
                "report_type":"version",
                "cli_version":env!("CARGO_PKG_VERSION"),
                "target":target,
                "build_identity":build_identity,
                "check_protocol_major":1,
                "rulepack_compatibility":"unverified",
            })
        );
    } else {
        println!("codeguard {}", env!("CARGO_PKG_VERSION"));
        println!("目标平台：{target}");
        println!("检查协议 major：1");
        println!(
            "构建来源声明：{}；规则包兼容范围：未验证",
            build_identity.unwrap_or("未提供")
        );
    }
    ExitCode::SUCCESS
}

fn detect(args: &[String]) -> ExitCode {
    let (path, json) = match parse_detect_args(args) {
        Ok(value) => value,
        Err(message) => {
            eprintln!("{message}");
            return ExitCode::from(2);
        }
    };
    let registry = match legacy_registry() {
        Ok(registry) => registry,
        Err(error) => {
            eprintln!("内置语言清单损坏：{error}");
            return ExitCode::from(4);
        }
    };
    let report = discover(&path, &registry, &NativeObservation);
    if json {
        println!("{}", report.to_json());
    } else {
        println!("项目：{}", report.root);
        println!(
            "观察完整：{}；已观察路径：{}",
            report.observation_complete, report.observed_entries
        );
        println!(
            "普通扫描点前缀排除根：{}；依赖目录排除根：{}；配置例外文件：{}；入库安全：未评估",
            report.dot_prefix_roots_excluded,
            report.dependency_roots_excluded,
            report.configuration_exception_files_observed
        );
        for (language, evidence) in &report.languages {
            println!(
                "{language}: 源文件 {}，清单 {}",
                evidence.source_files.len(),
                evidence.manifests.len()
            );
        }
        for blocked in &report.blocked_paths {
            println!("未读取：{blocked}");
        }
        for unknown in &report.unknown_conditions {
            println!("待解析：{unknown}");
        }
        for checker in &report.checker_configurations {
            println!(
                "{} [{}]：{}（{}）；下一步：{}",
                checker.checker_id,
                checker.category,
                checker.configuration,
                checker.configuration_ref,
                checker.next_action
            );
        }
        for tool in &report.native_tool_candidates {
            println!(
                "原生工具候选 {} [{:?}]：{}；版本：{}；下一步：{}（未执行）",
                tool.checker_id,
                tool.build_root,
                tool.state,
                tool.observed_version.as_deref().unwrap_or("未知"),
                tool.next_action
            );
        }
    }
    if report.observation_complete {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(3)
    }
}

fn parse_detect_args(args: &[String]) -> Result<(PathBuf, bool), String> {
    let mut path = None;
    let mut json = false;
    let mut index = 0;
    while index < args.len() {
        let current = &args[index];
        if current == "--format" {
            index += 1;
            let value = args.get(index).ok_or("--format 缺少值")?;
            if value != "json" && value != "human" {
                return Err(format!("不支持的格式：{value}"));
            }
            json = value == "json";
        } else if let Some(value) = current.strip_prefix("--format=") {
            if value != "json" && value != "human" {
                return Err(format!("不支持的格式：{value}"));
            }
            json = value == "json";
        } else if current.starts_with('-') || path.is_some() {
            return Err(format!("不支持的参数：{current}"));
        } else {
            path = Some(PathBuf::from(current));
        }
        index += 1;
    }
    Ok((path.unwrap_or_else(|| PathBuf::from(".")), json))
}

fn capabilities(args: &[String]) -> ExitCode {
    let query = match parse_capabilities_args(args) {
        Ok(value) => value,
        Err(message) => {
            eprintln!("{message}");
            return ExitCode::from(2);
        }
    };
    capabilities_with_registry(query, legacy_registry())
}

fn capabilities_with_registry(
    query: CapabilitiesArgs,
    registry_result: Result<LegacyRegistry, String>,
) -> ExitCode {
    let registry = match registry_result {
        Ok(registry) => registry,
        Err(error) => {
            eprintln!("内置语言清单损坏：{error}");
            return ExitCode::from(4);
        }
    };
    let selected: Vec<_> = registry
        .languages
        .iter()
        .filter(|entry| query.language.as_ref().is_none_or(|id| id == &entry.id))
        .collect();
    if selected.is_empty() {
        eprintln!("未知语言 ID：{}", query.language.expect("已确认选择为空"));
        return ExitCode::from(2);
    }
    if query.json && query.platform.is_none() && query.category.is_none() {
        let rows: Vec<_> = selected.iter().map(|entry| capability_row(entry)).collect();
        println!(
            "{}",
            serde_json::json!({"schema_version":"0.2.0","report_type":"capability_inventory","release_version":env!("CARGO_PKG_VERSION"),"languages":rows})
        );
    } else if query.json {
        let mut cells = Vec::new();
        for entry in &selected {
            let row = capability_row(entry);
            for (platform, categories) in &row.platforms {
                if query
                    .platform
                    .as_deref()
                    .is_some_and(|chosen| chosen != *platform)
                {
                    continue;
                }
                for (category, cell) in categories {
                    if query
                        .category
                        .as_deref()
                        .is_some_and(|chosen| chosen != *category)
                    {
                        continue;
                    }
                    cells.push(serde_json::json!({
                        "language":entry.id,
                        "legacy_status":entry.status,
                        "platform":platform,
                        "category":category,
                        "status":cell.status,
                        "reason":cell.reason,
                        "verified_combinations":[],
                    }));
                }
            }
        }
        println!(
            "{}",
            serde_json::json!({
                "schema_version":"0.2.0",
                "report_type":"capability_selection",
                "release_version":env!("CARGO_PKG_VERSION"),
                "cells":cells,
            })
        );
    } else if query.platform.is_none() && query.category.is_none() {
        for entry in selected {
            println!(
                "{}: 六类别×五平台检查能力均未验证（旧状态：{}）",
                entry.id, entry.status
            );
        }
    } else {
        for entry in selected {
            let row = capability_row(entry);
            for (platform, categories) in &row.platforms {
                if query
                    .platform
                    .as_deref()
                    .is_some_and(|chosen| chosen != *platform)
                {
                    continue;
                }
                for (category, cell) in categories {
                    if query
                        .category
                        .as_deref()
                        .is_some_and(|chosen| chosen != *category)
                    {
                        continue;
                    }
                    println!(
                        "{}/{}/{}: {} ({})",
                        entry.id, platform, category, cell.status, cell.reason
                    );
                }
            }
        }
    }
    ExitCode::SUCCESS
}

#[derive(Debug, Eq, PartialEq)]
struct CapabilitiesArgs {
    language: Option<String>,
    platform: Option<String>,
    category: Option<String>,
    json: bool,
}

fn parse_capabilities_args(args: &[String]) -> Result<CapabilitiesArgs, String> {
    let mut language = None;
    let mut platform = None;
    let mut category = None;
    let mut json = false;
    let mut index = 0;
    while index < args.len() {
        let current = &args[index];
        if current == "--platform" || current == "--category" {
            index += 1;
            let value = args.get(index).ok_or_else(|| format!("{current} 缺少值"))?;
            if current == "--platform" {
                platform = Some(value.clone());
            } else {
                category = Some(value.clone());
            }
        } else if let Some(value) = current.strip_prefix("--platform=") {
            platform = Some(value.into());
        } else if let Some(value) = current.strip_prefix("--category=") {
            category = Some(value.into());
        } else if current == "--format" {
            index += 1;
            let value = args.get(index).ok_or("--format 缺少值")?;
            if value != "json" && value != "human" {
                return Err(format!("不支持的格式：{value}"));
            }
            json = value == "json";
        } else if let Some(value) = current.strip_prefix("--format=") {
            if value != "json" && value != "human" {
                return Err(format!("不支持的格式：{value}"));
            }
            json = value == "json";
        } else if current.starts_with('-') || language.is_some() {
            return Err(format!("不支持的参数：{current}"));
        } else {
            language = Some(current.clone());
        }
        index += 1;
    }
    if let Some(value) = &platform {
        if !CANDIDATE_PLATFORMS.contains(&value.as_str()) {
            return Err(format!("未知平台 ID：{value}"));
        }
    }
    if let Some(value) = &category {
        if !CHECK_CATEGORIES.contains(&value.as_str()) {
            return Err(format!("未知类别 ID：{value}"));
        }
    }
    Ok(CapabilitiesArgs {
        language,
        platform,
        category,
        json,
    })
}

#[cfg(test)]
mod tests {
    use super::{
        CapabilitiesArgs, capabilities_with_registry, parse_capabilities_args, parse_detect_args,
    };
    use std::process::ExitCode;

    #[test]
    fn rejects_unsupported_format_before_any_observation() {
        assert!(parse_capabilities_args(&["--format".into(), "yaml".into()]).is_err());
    }

    #[test]
    fn accepts_language_and_json_format() {
        let actual = parse_capabilities_args(&["java".into(), "--format".into(), "json".into()]);
        assert_eq!(
            actual,
            Ok(CapabilitiesArgs {
                language: Some("java".into()),
                platform: None,
                category: None,
                json: true,
            })
        );
    }

    #[test]
    fn capability_filters_reject_unknown_dimensions_before_registry_use() {
        assert!(parse_capabilities_args(&["--platform=darwin".into()]).is_err());
        assert!(parse_capabilities_args(&["--category=style".into()]).is_err());
    }

    #[test]
    fn detect_rejects_invalid_arguments_before_file_observation() {
        assert!(parse_detect_args(&["--format".into(), "yaml".into()]).is_err());
        assert!(parse_detect_args(&["a".into(), "b".into()]).is_err());
    }

    #[test]
    fn corrupt_release_registry_is_internal_error() {
        let query = CapabilitiesArgs {
            language: None,
            platform: None,
            category: None,
            json: true,
        };
        assert_eq!(
            capabilities_with_registry(query, Err("corrupt".into())),
            ExitCode::from(4)
        );
    }
}
