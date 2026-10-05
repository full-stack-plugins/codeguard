//! ShellCheck 原生单文件入口；配置冻结后执行静态工具，不执行被检查脚本或自动安装。
use crate::{
    check_budget::{budget_record, select_check_timeout},
    shell_lint_arguments::ShellLintArguments,
    shellcheck_config::ShellCheckConfig,
    shellcheck_probe,
};
use codeguard_runtime::read_bounded_regular_file;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    process::ExitCode,
    sync::atomic::AtomicBool,
    time::{Duration, Instant},
};
/// 检查一个冻结Shell文件；参数为方言、配置/工具和预算，返回局部反馈及原工具修复指引。
pub fn run(args: &[String]) -> ExitCode {
    let a = match ShellLintArguments::parse(args) {
        Ok(a) => a,
        Err(e) => {
            eprintln!("{e}");
            return ExitCode::from(2);
        }
    };
    let (timeout, budget_source) = match select_check_timeout(a.timeout) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("{e}");
            return ExitCode::from(2);
        }
    };
    let deadline = Instant::now() + Duration::from_millis(timeout);
    let explicit = a.tool.is_some();
    let tool = a.tool.or_else(discover_tool);
    let resolved = a.file.canonicalize().ok();
    let source = read_bounded_regular_file(&a.file, 1024 * 1024)
        .ok()
        .filter(|b| std::str::from_utf8(b).is_ok());
    let config = resolved
        .as_deref()
        .map(|p| ShellCheckConfig::capture(p, a.config.as_deref()));
    let config_report = match &config {
        Some(Ok(c)) => c.report(),
        Some(Err(reason)) => {
            json!({"status":if *reason=="shellcheck_external_sources_unverified"{"unknown"}else{"invalid"},"source_path":null,"sha256":null,"reason":reason,"global_configuration":"not_loaded"})
        }
        None => {
            json!({"status":"unknown","source_path":null,"sha256":null,"reason":"source_unavailable","global_configuration":"not_loaded"})
        }
    };
    let known_unsupported_file = a
        .file
        .extension()
        .and_then(|s| s.to_str())
        .is_some_and(|s| matches!(s, "zsh" | "fish"))
        || a.file
            .file_name()
            .and_then(|s| s.to_str())
            .is_some_and(|s| {
                matches!(
                    s,
                    ".zshrc" | ".zshenv" | ".zprofile" | ".zlogin" | ".zlogout"
                )
            });
    let dialect_supported = !known_unsupported_file
        && matches!(
            a.dialect.as_str(),
            "sh" | "bash" | "dash" | "ksh" | "busybox"
        )
        && source.as_ref().is_none_or(|b| supported_shebang(b));
    let mut native = if !dialect_supported {
        shellcheck_probe::unavailable("shell_dialect_unsupported")
    } else {
        match (
            resolved.as_deref(),
            source.as_deref(),
            &config,
            tool.as_deref(),
        ) {
            (_, _, Some(Err(reason)), _) => shellcheck_probe::unavailable(reason),
            (Some(path), Some(bytes), Some(Ok(config)), Some(tool)) => shellcheck_probe::observe(
                tool,
                path,
                bytes,
                &a.dialect,
                config,
                deadline,
                &AtomicBool::new(false),
            ),
            (_, Some(_), _, None) => shellcheck_probe::unavailable("shellcheck_tool_not_found"),
            _ => shellcheck_probe::unavailable("shell_source_unavailable_or_invalid"),
        }
    };
    let stable = resolved
        .as_deref()
        .is_some_and(|p| a.file.canonicalize().ok().as_deref() == Some(p))
        && source.as_ref().is_some_and(|b| {
            read_bounded_regular_file(&a.file, 1024 * 1024).is_ok_and(|actual| actual == *b)
        })
        && config.as_ref().is_some_and(|r| {
            r.as_ref()
                .is_ok_and(|c| resolved.as_deref().is_some_and(|p| c.current(p)))
        });
    if source.is_some() && !stable && native["version"].is_string() {
        native = shellcheck_probe::unavailable("shellcheck_inputs_changed_during_check");
    }
    let cancelled = codeguard_runtime::sigint_cancellation_requested()
        || native["reason"] == "request_cancelled";
    let mut recheck = vec![
        "codeguard".to_string(),
        "lint".into(),
        "shell".into(),
        a.file.to_string_lossy().into_owned(),
        "--dialect".into(),
        a.dialect.clone(),
        "--format=json".into(),
    ];
    if let Some(p) = &tool {
        recheck.extend(["--shellcheck-tool".into(), p.to_string_lossy().into_owned()]);
    }
    if let Some(p) = &a.config {
        recheck.extend([
            "--shellcheck-config".into(),
            p.to_string_lossy().into_owned(),
        ]);
    }
    let source_sha = source.as_ref().map(|b| format!("{:x}", Sha256::digest(b)));
    let briefs:Vec<Value>=native["diagnostics"].as_array().into_iter().flatten().map(|d|{
        let actionable=stable && native["status"]=="diagnostics_observed";
        json!({"status":if actionable{"repair_guidance"}else{"investigation_required"},"evidence":{"source_path":a.file,"source_sha256":source_sha,"diagnostic":d},"rule_basis":format!("https://www.shellcheck.net/wiki/{}",d["rule_id"].as_str().unwrap_or("unknown")),"allowed_paths":if actionable{vec![json!(a.file)]}else{Vec::<Value>::new()},"steps":["先核对所选方言、原生配置及规则适用性；未完成时先修复环境或依赖，不能据旧位置修改源码。","按原生范围和官方规则检查引用、展开及可移植性，保持原行为；不要通过关闭规则或删任务冒充修复。","使用相同工具、方言和配置复检，记录差异；原生零诊断不等于完整项目通过。"],"recheck_argv":recheck,"attempt_history":[],"history_status":"not_integrated","closure_conditions":["原工具在稳定输入和有效原规则下复检，问题确实消失。","持久任务、批准白名单及完整项目门禁需要独立验收；本简报不能关闭任务。"]})
    }).collect();
    let mut report = json!({"schema_version":"0.1.0","report_type":"shell_lint_feedback","operation":"lint","language":"shell","source_path":a.file,"source_sha256":source_sha,"input_stable":stable,"dialect":a.dialect,"scope":"single_frozen_shell_file","command_status":if cancelled{"cancelled"}else{"incomplete"},"exit_code":if cancelled{130}else{3},"authority":"local_unverified","coverage_proven":false,"delivery_decision":"not_evaluated","execution_budget":budget_record(timeout,budget_source),"tool_selection":{"source":if explicit{"explicit"}else if tool.is_some(){"path"}else{"not_found"},"executable":tool},"native_identity_scope":"entry_only","project_configuration":config_report,"native":native,"native_column_unit":"unicode_scalar_one_based_tabs_one","syntax_precheck":{"status":"not_run","reason":"shell_wasm_unavailable"},"setup":{"native_tool_requirement":if matches!(native["status"].as_str(),Some("completed"|"diagnostics_observed")){"available"}else{"required"},"automatic_installation":false},"repair_briefs":briefs,"task_workflow_status":"not_integrated","next_actions":["准备ShellCheck0.11.0和适用方言；不支持的zsh/fish必须使用专用原生检查能力。","环境/配置/源依赖未完成时先恢复检查能力；部分诊断保留供调查。","持久任务、项目全范围、Dockerfile/IaC、安全及发行仍须检查。"]});
    crate::shell_lint_workbench::connect(&a.file, a.config.as_deref(), &mut report, deadline);
    if a.json {
        println!("{report}");
    } else {
        println!(
            "ShellCheck：{}；原因 {}；配置 {}；交付未评估。",
            report["native"]["status"],
            report["native"]["reason"],
            report["project_configuration"]["status"]
        );
        for d in report["native"]["diagnostics"]
            .as_array()
            .into_iter()
            .flatten()
        {
            println!(
                "{}:{}:{} {}（原生json1字符列，制表符计1）",
                report["source_path"], d["line"], d["column"], d["rule_id"]
            );
        }
        println!("修复指引：{}", report["repair_briefs"]);
        if report["workbench"].is_object() {
            println!("工作台：{}", report["workbench"]);
        }
        println!("下一步：{}", report["next_actions"]);
    }
    ExitCode::from(if cancelled { 130 } else { 3 })
}
fn discover_tool() -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    std::env::split_paths(&path)
        .filter(|p| p.is_absolute())
        .map(|p| p.join("shellcheck"))
        .find(|p| {
            std::fs::metadata(p).is_ok_and(|m| m.is_file() && m.permissions().mode() & 0o111 != 0)
        })
}
fn supported_shebang(source: &[u8]) -> bool {
    let text = std::str::from_utf8(source).unwrap_or("");
    let first = text.lines().next().unwrap_or("");
    let Some(rest) = first.strip_prefix("#!") else {
        return true;
    };
    let mut words = rest.split_whitespace();
    let Some(first) = words.next() else {
        return false;
    };
    let mut name = Path::new(first)
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("");
    if name == "env" {
        name = words
            .find(|w| !matches!(*w, "-S" | "--") && !w.contains('='))
            .unwrap_or("");
    }
    matches!(name, "sh" | "bash" | "dash" | "ksh" | "busybox")
}
