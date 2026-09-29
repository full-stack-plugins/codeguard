//! ESLint 局部原生反馈；当前不签发完整项目覆盖或持久任务关闭。
use crate::{
    doctor_scratch::DoctorScratch, eslint_lint_arguments::EslintLintArguments,
    eslint_probe::run_eslint_probe, eslint_probe_request::EslintProbeRequest,
};
use codeguard_adapters::EslintCommand;
use codeguard_runtime::read_bounded_regular_file;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    path::Path,
    process::ExitCode,
    sync::atomic::AtomicBool,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

/// 执行显式文件或目录 ESLint 并向 human/JSON 输出原生规则和准备动作。
/// 未提供上下文或完整策略时保持未完成；不安装工具、不构造默认规则。
pub fn run(args: &[String]) -> ExitCode {
    let args = match EslintLintArguments::parse(args) {
        Ok(value) => value,
        Err(reason) => {
            eprintln!("{reason}");
            return ExitCode::from(2);
        }
    };
    let deadline = Instant::now() + Duration::from_millis(args.timeout_ms);
    if args.config_map.is_some()
        && !std::fs::symlink_metadata(&args.source).is_ok_and(|m| m.is_dir())
    {
        eprintln!("--config-map 仅用于显式目录检查");
        return ExitCode::from(2);
    }
    if std::fs::symlink_metadata(&args.source).is_ok_and(|m| m.is_dir())
        && args.node.is_some()
        && args.entry.is_some()
        && args.config.is_some()
        && args.version.is_some()
        && args.cwd.is_some()
    {
        return crate::eslint_directory::run(&args, deadline);
    }
    #[cfg(feature = "wasm-precheck")]
    if crate::typescript_syntax_precheck::eligible(&args) {
        if let Some(candidate) =
            crate::eslint_native_first_candidate::observed_candidate(&args.source)
        {
            let mut native_args = args.clone();
            native_args.entry = Some(candidate.entry);
            native_args.config = Some(candidate.config);
            native_args.cwd = Some(candidate.root);
            native_args.version = Some(candidate.version);
            let report = if let Some(node) = crate::eslint_native_first_candidate::node_on_path() {
                native_args.node = Some(node);
                observe_with_preparation(&native_args, deadline)
            } else {
                let mut report = observe_with_preparation(&args, deadline);
                if report["reason"] == "eslint_execution_context_missing"
                    && report["workbench"]["next"]["repair_brief"]["step"].is_null()
                {
                    report["next_action"] = json!(
                        "项目本地 ESLint 候选和配置已发现；当前未解析到 Node，请提供 Node 路径并执行原生检查。候选尚未通过探测，不能认定 lint 已完成"
                    );
                }
                report
            };
            if args.json {
                println!("{report}");
            } else {
                print_feedback(&report);
            }
            return ExitCode::from(if report["reason"] == "request_cancelled" {
                130
            } else {
                3
            });
        }
        let report = crate::typescript_syntax_precheck::observe(&args, deadline);
        if args.json {
            println!("{report}");
        } else {
            print_feedback(&report);
        }
        return ExitCode::from(3);
    }
    let report = observe_with_preparation(&args, deadline);
    if args.json {
        println!("{report}");
    } else {
        print_feedback(&report);
    }
    ExitCode::from(if report["reason"] == "request_cancelled" {
        130
    } else {
        3
    })
}
pub(crate) fn observe_with_preparation(args: &EslintLintArguments, deadline: Instant) -> Value {
    let mut report = observe(args, deadline);
    // 早期上下文失败也进入环境待办；取消、外部目标或不支持范围不能制造源码任务。
    if let (Some(workspace), Some(reason)) = (
        &args.workspace,
        report["reason"].as_str().filter(|r| {
            report["local_coherent"] == false && crate::eslint_preparation::valid_reason(r)
        }),
    ) {
        report["workbench"] = match workspace.canonicalize() {
            Ok(root) if root == *workspace => {
                crate::eslint_workbench::connect_preparation(&root, args, reason)
            }
            _ => json!({"status":"workspace_invalid"}),
        };
        report["workbench_status"] = report["workbench"]["status"].clone();
    }
    if let Some(step) = report["workbench"]["next"]["repair_brief"]["step"].as_str() {
        report["next_action"] = json!(step);
    }
    report
}
pub(crate) fn print_feedback(report: &Value) {
    println!(
        "ESLint 局部检查：{}；完整项目门禁尚未判定。",
        report["status"]
    );
    for finding in report["findings"].as_array().into_iter().flatten() {
        println!(
            "原生规则 {}，行 {}，列 {}，严重度 {}",
            finding["rule_id"], finding["line"], finding["column"], finding["severity"]
        );
    }
    if !report["reason"].is_null() {
        println!("待处理原因：{}", report["reason"]);
    }
    if let Some(precheck) = report.get("syntax_precheck") {
        println!(
            "内置语法初检：{}；疑似恢复节点 {}；grammar 版本/方言未验收，不等于原生 lint 通过。",
            precheck["status"],
            precheck["observations"].as_array().map_or(0, Vec::len)
        );
        for observation in precheck["observations"]
            .as_array()
            .into_iter()
            .flatten()
            .take(8)
        {
            println!(
                "疑似语法 {}，行 {}，列 {}；须由适用原生工具确认。",
                observation["kind"], observation["start_line"], observation["start_column"]
            );
        }
    }
    println!("工作台：{}", report["workbench_status"]);
    println!("下一步：{}", report["next_action"]);
}
fn observe(args: &EslintLintArguments, deadline: Instant) -> Value {
    observe_captured(args, deadline, None)
}
pub(crate) fn observe_for_task(
    root: &Path,
    args: &EslintLintArguments,
    deadline: Instant,
) -> (Value, Option<Value>) {
    let mut scan = None;
    let feedback = observe_captured(args, deadline, Some((root, &mut scan)));
    (feedback, scan)
}
fn observe_captured(
    args: &EslintLintArguments,
    deadline: Instant,
    capture: Option<(&Path, &mut Option<Value>)>,
) -> Value {
    let (Some(node), Some(entry), Some(config), Some(version), Some(cwd)) = (
        &args.node,
        &args.entry,
        &args.config,
        &args.version,
        &args.cwd,
    ) else {
        return feedback("eslint_execution_context_missing");
    };
    let source = match std::fs::symlink_metadata(&args.source) {
        Ok(metadata) if metadata.is_file() => match args.source.canonicalize() {
            Ok(path) => path,
            _ => return feedback("eslint_source_unavailable"),
        },
        _ => return feedback("eslint_scope_requires_explicit_file"),
    };
    if !source.extension().is_some_and(|extension| {
        ["js", "jsx", "mjs", "cjs", "ts", "tsx", "mts", "cts"]
            .iter()
            .any(|value| extension == *value)
    }) {
        return feedback("eslint_source_kind_not_supported");
    }
    // 临时区仅承载本轮原生报告；工作台另存脱敏观察，不声称保留完整原生证据。
    let id = format!(
        "eslint-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |value| value.as_nanos())
    );
    let Some(scratch) = DoctorScratch::create(&id) else {
        return feedback("eslint_private_workspace_unavailable");
    };
    let mut hashes = BTreeMap::new();
    for (path, limit) in [
        (node, 128 * 1024 * 1024),
        (entry, 16 * 1024 * 1024),
        (config, 1024 * 1024),
        (&source, 16 * 1024 * 1024),
    ] {
        if Instant::now() >= deadline {
            return feedback("request_deadline_exceeded");
        }
        let Ok(bytes) = read_bounded_regular_file(path, limit) else {
            return feedback("eslint_input_unavailable");
        };
        hashes.insert(path.clone(), Sha256::digest(bytes).into());
    }
    let request = EslintProbeRequest {
        command: EslintCommand {
            node: node.clone(),
            entry: entry.clone(),
            config: config.clone(),
            sources: vec![source],
            report: scratch.path().join(format!("{id}.json")),
            max_warnings: None,
        },
        cwd: cwd.clone(),
        evidence_dir: scratch.path().into(),
        run_id: id,
        expected_version: version.clone(),
        expected_sha256: hashes,
        deadline,
    };
    let result = run_eslint_probe(&request, &AtomicBool::new(false));
    if let Some((root, scan)) = capture {
        *scan = crate::eslint_workbench::prepare(root, &request, &result).ok();
    }
    let mut report = feedback(
        result
            .reason
            .unwrap_or("project_context_and_policy_unverified"),
    );
    report["local_coherent"] = json!(result.local_coherent);
    if result.local_coherent {
        report["status"] = json!("local_observation");
    }
    if let Some(workspace) = args.workspace.as_ref().filter(|_| result.local_coherent) {
        report["workbench"] = match workspace.canonicalize() {
            Ok(root) if root == *workspace => {
                crate::eslint_workbench::connect(&root, &request, &result)
            }
            _ => json!({"status":"workspace_invalid"}),
        };
        report["workbench_status"] = report["workbench"]["status"].clone();
    }
    if let Some(parsed) = result.parsed {
        report["suppressed_count"] = json!(parsed.suppressed_count);
        report["findings"]=json!(parsed.findings.iter().map(|finding|json!({"path":Path::new(&finding.path).file_name().and_then(|name|name.to_str()),"rule_id":finding.rule_id,"severity":finding.severity,"line":finding.line,"column":finding.column,"message_sha256":format!("{:x}",Sha256::digest(finding.message.as_bytes()))})).collect::<Vec<_>>());
    }
    report
}
fn feedback(reason: &str) -> Value {
    json!({"schema_version":"0.2.0","report_type":"eslint_local_feedback","status":"incomplete","local_coherent":false,"coverage_proven":false,"delivery_decision":"not_evaluated","reason":reason,"findings":[],"suppressed_count":0,"workbench_status":"not_connected","workbench":null,"next_action":match reason {
        "eslint_execution_context_missing"=>"提供显式 Node、原 ESLint JS 入口、具体版本、原工作目录与项目原 flat config 后复检；不安装或替换规则",
        "eslint_scope_requires_explicit_file"=>"当前入口需显式单文件；项目目录与完整源集调度尚未接通，不将目录标为已检查",
        "request_cancelled"|"request_deadline_exceeded"=>"重新安排原工具检查预算；中断不要求修改源码",
        _=>"核对原配置、parser/插件与本轮规则；按原生发现修复后用同一入口和配置复检，抑制及未知覆盖需核查，任务不能自行关闭",
    }})
}
