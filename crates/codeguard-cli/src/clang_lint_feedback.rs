//! 显式Clang标准请求的单文件原生反馈；完整项目模型及工作台原生导入尚待接线。
use crate::syntax_lint_arguments::SyntaxLintArguments;
use serde_json::{Value, json};
use std::time::Instant;

/// 保留原生Clang结果与输入身份；参数为已校验请求及共同截止时间，返回不具门禁权威的局部反馈。
pub(crate) fn observe(args: &SyntaxLintArguments, deadline: Instant) -> Value {
    let mut report = json!({"schema_version":"0.2.0","report_type":"syntax_lint_feedback","selection":{"category":"lint","language":args.language},
        "status":"incomplete","coverage_proven":false,"delivery_decision":"not_evaluated","findings":[],
        "native":{"status":"incomplete","reason":"clang_source_scope_unavailable","version":null,"tool_sha256":null,"diagnostics":[]},
        "native_context":{"standard":args.standard,"scope":"explicit_standalone_stdin","project_configuration":"unknown","warning_profile":"clang-wall-extra-pedantic-v1"},
        "source_sha256":null,"syntax_candidates":null,"syntax_tasks":null,"setup":{"requirement":"required","task_id":null},"reason":"native_project_coverage_unverified",
        "next_action":"按原工具、语言及明确标准复检；原生语法观察不替代项目clang-tidy、编译数据库、头文件/宏、安全或依赖检查。本入口尚未持久同步Clang原生任务，不凭零诊断关闭历史任务"});
    let source_path = args
        .source
        .canonicalize()
        .unwrap_or_else(|_| args.source.clone());
    let mut command = json!([
        "codeguard",
        "lint",
        args.language,
        source_path,
        "--clang-tool",
        args.clang_tool,
        "--standard",
        args.standard,
        "--format=json"
    ]);
    if let Some(workspace) = &args.workspace {
        command
            .as_array_mut()
            .expect("复检命令为参数数组")
            .extend([json!("--workspace"), json!(workspace)]);
    }
    report["path"] = json!(source_path);
    report["verification_command"] = command;
    let source = match crate::plain_syntax_source::read_plain_source(&args.source) {
        Ok(source) => source,
        Err(reason) => {
            report["native"]["reason"] = json!(reason);
            return report;
        }
    };
    let Ok(path) = args.source.canonicalize() else {
        return report;
    };
    if args.workspace.as_ref().is_some_and(|root| {
        root.canonicalize().ok().as_ref() != Some(root) || !path.starts_with(root)
    }) {
        return report;
    }
    if !((args.language == "c" && path.extension().is_some_and(|s| s == "c"))
        || (args.language == "cpp"
            && path
                .extension()
                .and_then(|s| s.to_str())
                .is_some_and(|s| matches!(s, "cpp" | "cc" | "cxx" | "C" | "CPP" | "cp" | "c++"))))
    {
        return report;
    }
    use sha2::{Digest, Sha256};
    report["source_sha256"] = json!(format!("{:x}", Sha256::digest(&source)));
    let mut native = crate::clang_syntax_probe::observe(
        args.clang_tool.as_deref().expect("明确Clang请求已核对工具"),
        &args.language,
        args.standard.as_deref().expect("明确Clang请求已核对标准"),
        &source,
        deadline,
    );
    if !crate::plain_syntax_source::read_plain_source(&args.source)
        .is_ok_and(|current| current == source)
    {
        native["status"] = json!("incomplete");
        native["reason"] = json!("clang_source_changed");
        native["diagnostics"] = json!([]);
    }
    report["native"] = native;
    report
}
