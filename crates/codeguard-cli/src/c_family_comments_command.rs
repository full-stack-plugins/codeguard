//! C/C++原生文档探针：固定Clang文档档案、冻结输入及准确规则分类，完整项目闭环仍待验收。
use crate::c_family_comments_arguments::CFamilyCommentsArguments;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    process::ExitCode,
    sync::atomic::AtomicBool,
    time::{Duration, Instant},
};

/// 执行显式单文件文档探针；参数为语言后的原始argv，返回错参2、局部未完成3或取消130。
pub fn run(args: &[String]) -> ExitCode {
    let request = match CFamilyCommentsArguments::parse(args) {
        Ok(request) => request.0,
        Err(reason) => {
            eprintln!("{reason}");
            return ExitCode::from(2);
        }
    };
    let deadline = Instant::now() + Duration::from_millis(request.timeout_ms);
    let workspace = match crate::c_family_comments_workbench::resolve_root(
        &request.source,
        request.workspace.as_deref(),
    ) {
        Ok(workspace) => workspace,
        Err(reason) if request.workspace.is_some() => {
            eprintln!("{reason}");
            return ExitCode::from(2);
        }
        Err(_) => None,
    };
    let path = request
        .source
        .canonicalize()
        .unwrap_or_else(|_| request.source.clone());
    let verification_command = json!([
        "codeguard",
        "comments",
        request.language,
        path,
        "--clang-tool",
        request.clang_tool,
        "--standard",
        request.standard,
        "--timeout",
        format!("{}ms", request.timeout_ms),
        "--format=json"
    ]);
    let mut native = json!({"status":"incomplete","reason":"clang_source_scope_unavailable","version":null,"tool_sha256":null,"diagnostics":[]});
    let mut source_sha256 = Value::Null;
    let mut structure = json!({"status":"incomplete","reason":"clang_structure_unavailable","observation":null,"native_raw_diagnostic_count":null});
    let source_observation = crate::plain_syntax_source::read_plain_source(&request.source);
    if let Ok(source) = &source_observation {
        let extension = path.extension().and_then(|extension| extension.to_str());
        let applicable = match request.language.as_str() {
            "c" => extension == Some("c"),
            "cpp" => extension.is_some_and(|extension| {
                matches!(extension, "cpp" | "cc" | "cxx" | "C" | "CPP" | "cp" | "c++")
            }),
            _ => false,
        };
        if applicable {
            source_sha256 = json!(format!("{:x}", Sha256::digest(source)));
            (native, structure) = crate::clang_syntax_probe::observe_documentation_with_structure(
                request.clang_tool.as_deref().expect("工具上下文已校验"),
                &request.language,
                request.standard.as_deref().expect("标准上下文已校验"),
                source,
                deadline,
                &AtomicBool::new(false),
            );
            if !crate::plain_syntax_source::read_plain_source(&request.source)
                .is_ok_and(|current| current == *source)
            {
                native["status"] = json!("incomplete");
                native["reason"] = json!("clang_source_changed");
                native["diagnostics"] = json!([]);
                structure = json!({"status":"incomplete","reason":"clang_source_changed","observation":null,"native_raw_diagnostic_count":null});
            }
        }
    } else if let Err(reason) = source_observation {
        native["reason"] = json!(reason);
    }
    let cancelled = codeguard_runtime::sigint_cancellation_requested();
    if cancelled || Instant::now() >= deadline {
        native["status"] = json!("incomplete");
        native["reason"] = json!(if cancelled {
            "request_cancelled"
        } else {
            "clang_execution_incomplete"
        });
        native["diagnostics"] = json!([]);
        structure = json!({"status":"incomplete","reason":if cancelled {"request_cancelled"}else{"clang_execution_incomplete"},"observation":null,"native_raw_diagnostic_count":null});
    }
    let mut documentation_findings = Vec::new();
    let mut unclassified = Vec::new();
    for row in native["diagnostics"].as_array().into_iter().flatten() {
        if let Some(mut guidance) = row["rule_id"]
            .as_str()
            .and_then(codeguard_adapters::clang_documentation_guidance)
        {
            let object = guidance.as_object_mut().expect("固定规则指引为对象");
            object.extend(row.as_object().expect("原生定位为对象").clone());
            object.insert("path".into(), json!(path));
            documentation_findings.push(guidance);
        } else {
            unclassified.push(row.clone());
        }
    }
    // 语法错误会限制文档AST覆盖；保留所有原生诊断，但不把失败解析当完整文档扫描。
    let has_native_errors = native["diagnostics"]
        .as_array()
        .into_iter()
        .flatten()
        .any(|row| row["level"] == "error");
    let complete = matches!(
        native["status"].as_str(),
        Some("completed" | "diagnostics_observed")
    ) && !has_native_errors;
    let exit = if cancelled { 130 } else { 3 };
    let mut report = json!({"schema_version":"0.1.0","report_type":"c_family_comments_feedback","operation":"comments","language":request.language,
        "command_status":if cancelled {"cancelled"} else {"incomplete"},"exit_code":exit,"authority":"local_unverified","coverage_proven":false,"delivery_decision":"not_evaluated",
        "local_scan_complete":complete,"detailed_contract_qualification":"not_granted","workspace_binding":"not_bound","next":null,
        "reason":if has_native_errors {"native_syntax_errors_limit_documentation"} else {"native_documentation_coverage_unverified"},
        "path":path,"source_sha256":source_sha256,"verification_command":verification_command,
        "documentation_configuration":{"origin":"explicit_probe_profile","profile":"clang-documentation-v1","standard":request.standard,"project_configuration":"unknown","comment_scope":"recognized_documentation_comments_only","rule_coverage":"limited"},
        "native":native,"documentation_findings":documentation_findings,"unclassified_native_diagnostics":unclassified,
        "next_actions":["按原生规则和位置修正文档，再使用本报告原工具与标准命令复检；完整项目文档政策和任务持久闭环仍须接入。",
            "Clang此档案不检查所有缺失注释或用途/异常/行为说明；零诊断不能证明详细文档合规，不能关闭历史任务或授予生产资格。"]});
    report["documentation_structure"] = structure.clone();
    if let Some(root) = workspace {
        crate::c_family_comments_workbench::connect(
            &root,
            request.clang_tool.as_deref().expect("工具已校验"),
            &mut report,
            deadline,
        );
        crate::c_family_structure_workbench::connect(
            &root,
            request.clang_tool.as_deref().expect("工具已校验"),
            &mut report,
            deadline,
        );
        report["next_actions"][0] = json!(
            "按当前原生规则和位置修正文档并运行原工具复扫；原警告稳定任务及局部task verify已接入；结构任务按工作台状态提供指引，专用结构task verify与可信关闭仍待实现。"
        );
    }
    report["schema_version"] = json!(if report.get("workbench").is_some() {
        "0.7.0"
    } else {
        "0.5.0"
    });
    report["documentation_structure"] = structure;
    if report.get("structural_workbench").is_none() {
        report["structural_task_workflow_status"] = json!("not_integrated");
    }
    report["next_actions"].as_array_mut().expect("固定反馈动作").push(json!("读取原生AST结构观察中的缺失文档/用途/参数/返回组件并依据真实API补充说明；已初始化工作区可读取结构任务的当前定位并运行原comments命令复扫；专用结构task verify与可信关闭仍待接线，不把非空说明当准确性或关闭证据。"));
    if request.json {
        println!("{report}");
    } else {
        println!(
            "{} 原生文档检查：局部完成 {}；详细文档覆盖未验证。",
            request.language, complete
        );
        println!(
            "原生状态 {}；原因 {}",
            report["native"]["status"], report["native"]["reason"]
        );
        println!(
            "原生文档结构 {}；原因 {}",
            report["documentation_structure"]["status"],
            report["documentation_structure"]["reason"]
        );
        for row in report["documentation_structure"]["observation"]["functions"]
            .as_array()
            .into_iter()
            .flatten()
            .take(8)
        {
            println!(
                "文档结构 {}：{}:{}:{}；缺失组件 {}；状态 {}",
                row["name"],
                report["path"],
                row["line"],
                row["column_byte"],
                row["missing_components"],
                row["structure_status"]
            );
        }
        if !report["workbench"].is_null() {
            println!(
                "工作台 {}；任务 {}",
                report["workbench"]["status"], report["workbench"]["task_ids"]
            );
        }
        for row in report["documentation_findings"]
            .as_array()
            .into_iter()
            .flatten()
            .take(8)
        {
            println!(
                "原生规则 {}：{}:{}:{}；{}",
                row["rule_id"], row["path"], row["line"], row["column_byte"], row["rule_summary"]
            );
            for step in row["repair_steps"].as_array().into_iter().flatten() {
                println!("{}", step.as_str().unwrap_or("复核原文档契约"));
            }
        }
        for row in report["unclassified_native_diagnostics"]
            .as_array()
            .into_iter()
            .flatten()
            .take(8)
        {
            println!(
                "未分类原生诊断 {}：{}:{}；尚无文档规则适配",
                row["rule_id"], row["line"], row["column_byte"]
            );
        }
        for action in report["next_actions"].as_array().into_iter().flatten() {
            println!("{}", action.as_str().unwrap_or("检查尚未完成"));
        }
    }
    ExitCode::from(exit)
}
