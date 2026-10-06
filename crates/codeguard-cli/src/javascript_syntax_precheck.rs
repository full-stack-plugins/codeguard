//! JavaScript 独立入口复用项目候选扫描与稳定任务；不替代原生 ESLint。
use crate::eslint_lint_arguments::EslintLintArguments;
use serde_json::{Value, json};
use std::{collections::BTreeSet, time::Instant};

/// 判断请求是否属于 JavaScript grammar 的整文件输入；不推断 module/CommonJS 语义。
pub(crate) fn supported(args: &EslintLintArguments) -> bool {
    args.source.extension().is_some_and(|extension| {
        ["js", "mjs", "cjs", "jsx"]
            .iter()
            .any(|value| extension == *value)
    })
}

/// 在原生上下文发现为空后，以同一截止时间观察单文件并同步既有确认任务。
/// 参数为原请求及截止时间，返回无交付权威的版本化候选反馈。
pub(crate) fn observe(args: &EslintLintArguments, deadline: Instant) -> Value {
    let mut report = json!({"schema_version":"0.6.0","report_type":"eslint_local_feedback",
        "status":"incomplete","local_coherent":false,"coverage_proven":false,
        "delivery_decision":"not_evaluated","reason":"eslint_execution_context_missing",
        "findings":[],"suppressed_count":0,"workbench_status":"not_connected","workbench":null,
        "native":{"status":"not_run","reason":"explicit_context_missing"},
        "setup":{"requirement":"required","reason":"native_confirmation_needed","task_id":null},
        "syntax_candidates":{"status":"not_run","reason":"source_scope_unavailable","observations":[]},
        "syntax_tasks":null,
        "next_action":"源码范围无法绑定；先核对源码位置及 --workspace 的已核验绝对路径，本次语法初检未运行，不修改源码"});
    let source = match args.source.canonicalize() {
        Ok(source) => source,
        Err(_) => return report,
    };
    let root = match args.workspace.as_deref().or_else(|| source.parent()) {
        Some(path) => match path.canonicalize() {
            Ok(root)
                if args
                    .workspace
                    .as_ref()
                    .is_none_or(|workspace| *workspace == root) =>
            {
                root
            }
            _ => return report,
        },
        None => return report,
    };
    let Some(relative) = source
        .strip_prefix(&root)
        .ok()
        .and_then(|path| path.to_str())
    else {
        return report;
    };
    let selected = BTreeSet::from([relative.to_owned()]);
    let empty = Value::Null;
    let coverage = crate::rust_native_syntax_coverage::RustNativeSyntaxCoverage::default();
    let syntax = crate::check_syntax_candidates::observe_selected(
        &root,
        &selected,
        crate::check_syntax_candidates::NativeCoverage {
            node_lint: &empty,
            python_lint: &empty,
            go_lint: &empty,
            erlang_lint: &empty,
            kotlin_lint: &empty,
            zig_lint: &empty,
            swift_lint: &empty,
            ruby_lint: &empty,
            rust_targets: &coverage,
            go_tool: None,
        },
        1,
        deadline,
        None,
    );
    report["next_action"] = json!(
        "查看本次候选或检查未完成原因，恢复适用的原生 JavaScript/ESLint 上下文确认；不凭候选直接修改源码或关闭任务"
    );
    // 只有已完成本次有界扫描且没有任何候选，才将安装建议降为推荐；语法能力仍未验收。
    let no_candidates = syntax["status"] == "observed_partial"
        && syntax["skipped_count"] == 0
        && syntax["unrouted_count"] == 0
        && syntax["observations"].as_array().is_some_and(|rows| {
            rows.len() == 1
                && rows[0]["status"] == "candidate_observed"
                && rows[0]["reason"].is_null()
                && rows[0]["recovery_count"] == 0
                && rows[0]["structural_observation_count"]
                    .as_u64()
                    .unwrap_or(0)
                    == 0
        });
    if no_candidates {
        report["setup"]["requirement"] = json!("recommended");
        report["setup"]["reason"] = json!("native_lint_recommended");
        report["next_action"] = json!(
            "本次有界候选初检未发现疑似问题；推荐配置并运行原生 ESLint，类型、依赖、安全及完整项目质量仍未检查"
        );
    }
    if codeguard_runtime::sigint_cancellation_requested() {
        report["reason"] = json!("request_cancelled");
    } else if args.workspace.is_some() {
        let mut tasks = crate::syntax_confirmation::persist(&root, &syntax, deadline);
        let pending = if no_candidates {
            match pending_confirmation(&root, relative) {
                Ok(task) => task,
                Err(reason) => {
                    // 无法读取历史不能等价于没有待办；保留原生义务，输出固定恢复原因。
                    report["setup"]["requirement"] = json!("required");
                    report["setup"]["reason"] = json!("native_confirmation_needed");
                    tasks["status"] = json!("incomplete");
                    tasks["failures"]
                        .as_array_mut()
                        .expect("同步结果包含失败数组")
                        .push(json!({"path":relative,"language":"javascript","reason":reason}));
                    None
                }
            }
        } else {
            None
        };
        report["workbench_status"] = tasks["status"].clone();
        let mut workbench = json!({"status":tasks["status"]});
        let selected_task = tasks["tasks"][0]["task_id"]
            .as_str()
            .map(str::to_owned)
            .or(pending);
        if let Some(id) = selected_task {
            report["setup"]["task_id"] = json!(id);
            workbench["task_id"] = json!(id);
            if no_candidates {
                // 当前初检没有候选，不代表历史原生确认已完成；保留待办及明确安装要求。
                report["setup"]["requirement"] = json!("required");
                report["setup"]["reason"] = json!("native_confirmation_still_pending");
            }
            if let Ok(brief) = crate::next_command::read_task_brief(&root, &id) {
                report["next_action"] = brief["step"].clone();
            }
        }
        if tasks["failures"]
            .as_array()
            .is_some_and(|failures| !failures.is_empty())
        {
            report["next_action"] = json!(if tasks["failures"].as_array().is_some_and(|failures| {
                failures
                    .iter()
                    .any(|failure| failure["reason"] == "workspace_not_initialized")
            }) {
                "工作区未初始化；先对原工作区执行 codeguard init <工作区> --apply，再按原 lint 命令复检。保留初检候选，不修改源码或虚构任务"
            } else {
                "本次候选任务同步未完成；查看 syntax_tasks.failures，核查 .codeguard/findings、tasks、reports 的路径、权限及受管工作区身份，恢复历史任务记录，再按原命令复检，不自行创建关闭证据"
            });
        }
        report["workbench"] = workbench;
        report["syntax_tasks"] = tasks;
    }
    report["syntax_candidates"] = syntax;
    report
}

// 仅从已初始化工作区及经过完整工作台校验的现存任务恢复引用，不制造新任务或关闭证据。
fn pending_confirmation(
    root: &std::path::Path,
    relative: &str,
) -> Result<Option<String>, &'static str> {
    let baseline = crate::workspace_refresh::read_workspace_baseline(root)
        .map_err(|_| "workspace_invalid")?
        .ok_or("workspace_not_initialized")?;
    let fingerprint = crate::eslint_preparation::fingerprint(
        baseline.workspace_id().ok_or("workspace_id_unavailable")?,
        relative,
    );
    let id = format!("CG-B-{}", &fingerprint[..32]);
    // 检查父目录本身，避免通过可替换链接读取其他工作区的历史。
    for path in [
        root.join(".codeguard"),
        root.join(".codeguard/findings"),
        root.join(".codeguard/tasks"),
    ] {
        match std::fs::symlink_metadata(&path) {
            Ok(metadata)
                if metadata.file_type().is_dir()
                    && path.canonicalize().ok().as_deref() == Some(path.as_path()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            _ => return Err("workspace_records_unavailable"),
        }
    }
    let directory = root.join(".codeguard/findings").join(&id);
    let projection = root.join(".codeguard/tasks").join(format!("{id}.md"));
    let absent = |path: &std::path::Path| -> Result<bool, &'static str> {
        match std::fs::symlink_metadata(path) {
            Ok(_) => Ok(false),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(true),
            Err(_) => Err("task_record_unavailable"),
        }
    };
    // 只有事实目录和投影均不存在才表示没有同范围任务；半缺失要求恢复记录。
    if absent(&directory)? && absent(&projection)? {
        return Ok(None);
    }
    let brief = crate::next_command::read_task_brief(root, &id)?;
    if brief["checker_id"] != "node.eslint.preparation"
        || brief["kind"] != "blocker"
        || brief["scope"] != relative
    {
        return Err("task_identity_conflict");
    }
    Ok(Some(id))
}

/// 输出有界脱敏语法与结构位置；参数为本轮候选报告，不输出源码片段或标识符。
pub(crate) fn print_feedback(syntax: &Value) {
    println!(
        "内置 JavaScript 候选初检：{}；原生 lint 尚未完成。",
        syntax["status"]
    );
    for row in syntax["observations"]
        .as_array()
        .into_iter()
        .flatten()
        .take(1)
    {
        println!(
            "疑似恢复节点 {}，结构候选 {}；检查原因 {}。",
            row["recovery_count"],
            row["structural_observation_count"].as_u64().unwrap_or(0),
            row["reason"]
        );
        for recovery in row["recoveries"].as_array().into_iter().flatten().take(8) {
            println!(
                "疑似语法 {}，零起始行 {}、UTF-8 字节列 {}；须原生确认。",
                recovery["kind"], recovery["start_row"], recovery["start_column_byte"]
            );
        }
        for structure in row["structural_observations"]
            .as_array()
            .into_iter()
            .flatten()
            .take(8)
        {
            println!(
                "疑似结构规则 {}，零起始行 {}、UTF-8 字节列 {}；须原生确认。",
                structure["rule_id"], structure["start_row"], structure["start_column_byte"]
            );
        }
    }
}
