//! 从本地持久事实构造只读修复简报；不执行仓库内容，也不签发门禁。

use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use serde_json::{Value, json};
use sha2::{Digest, Sha256};

use crate::task_verify_command::{
    classify, classify_cve, classify_doctor, classify_go, classify_java, classify_rust,
};
use crate::workspace_refresh::read_workspace_baseline;

const MAX_FACT_BYTES: u64 = 128 * 1024;
const MAX_SOURCE_BYTES: u64 = 16 * 1024 * 1024;

struct Arguments {
    root: PathBuf,
    json: bool,
}

struct Candidate {
    id: String,
    priority: u8,
    brief: Value,
}

struct VerificationObservation {
    outcome: String,
    run_id: String,
    report_sha256: String,
    source_sha256: Option<String>,
    config_sha256: Option<String>,
    checkstyle_config_path: Option<String>,
    checkstyle_source_path: Option<String>,
    checkstyle_reason: Option<String>,
    checkstyle_tool_inputs: Option<Value>,
    ruff_configuration: Option<(String, String)>,
    go_module_identities: Option<Value>,
    go_task_scope: Option<Value>,
    go_source_snapshot_sha256: Option<String>,
}

/// 给出本地下一步；退出零只说明只读查询完成，不代表质量通过。
pub fn run(args: &[String]) -> ExitCode {
    let parsed = match parse_args(args) {
        Ok(parsed) => parsed,
        Err(message) => {
            eprintln!("{message}");
            return ExitCode::from(2);
        }
    };
    let root = match parsed.root.canonicalize() {
        Ok(root) if root.is_dir() => root,
        _ => return print_error(parsed.json, "project_unreadable"),
    };
    let report = match read_local_brief(&root) {
        Ok(view) => view,
        Err(reason) => return print_error(parsed.json, reason),
    };
    if parsed.json {
        println!("{report}");
    } else {
        println!(
            "CodeGuard 下一步：{}；{}",
            report["disposition"].as_str().unwrap_or("unknown"),
            report["reason"].as_str().unwrap_or("unknown")
        );
        if let Some(brief) = report["repair_brief"].as_object() {
            println!(
                "任务 {}：{}；范围 {}；复检 {}",
                brief["task_id"], brief["step"], brief["scope"], brief["recheck_argv"]
            );
        } else {
            println!("建议：{}", report["next_actions"]);
        }
        if report["repair_brief"].is_object()
            && report["next_actions"]
                .as_array()
                .is_some_and(|actions| !actions.is_empty())
        {
            println!("保留待处理任务的只读查询：{}", report["next_actions"]);
        }
        println!("本命令只读取本地待办，交付门禁未评估。");
    }
    ExitCode::SUCCESS
}

/// 读取当前工作区的局部下一步视图，供扫描完成后的对话反馈复用。
///
/// 参数 `root` 是已规范化项目根；返回值仍为本地未验证视图，不包含交付许可。
pub fn read_local_brief(root: &Path) -> Result<Value, &'static str> {
    build_view(root, None, None)
}

/// 读取指定原生检查器的下一步；其它检查器的任务仍校验，但不会被推荐给局部检查。
pub(crate) fn read_local_brief_for_checker(
    root: &Path,
    checker_id: &str,
) -> Result<Value, &'static str> {
    build_view(root, Some(checker_id), None)
}

/// 从本轮已同步的任务身份读取下一步；返回既有 next 协议，不扩大到其它历史任务。
pub(crate) fn read_local_brief_for_tasks(
    root: &Path,
    ids: &std::collections::BTreeSet<String>,
) -> Result<Value, &'static str> {
    if ids.is_empty() {
        return Ok(Value::Null);
    }
    build_view(root, None, Some(ids))
}

/// 读取指定本地任务的受限事实简报，供原工具复检确定范围。
///
/// 参数 `id` 只能是稳定 CG 身份；返回值不从可编辑 Markdown 提取指令。
pub fn read_task_brief(root: &Path, id: &str) -> Result<Value, &'static str> {
    read_task_brief_inner(root, id, false)
}

/// 为缺失的可读投影读取结构化指引；只豁免文件不存在，不豁免事实或链接校验。
pub(crate) fn read_task_brief_for_projection(root: &Path, id: &str) -> Result<Value, &'static str> {
    read_task_brief_inner(root, id, true)
}

fn read_task_brief_inner(
    root: &Path,
    id: &str,
    allow_missing_projection: bool,
) -> Result<Value, &'static str> {
    if !safe_id(id) {
        return Err("task_id_invalid");
    }
    let baseline = read_workspace_baseline(root).map_err(|reason| {
        if reason == "legacy_workspace_requires_manual_migration" {
            "legacy_workspace_requires_manual_migration"
        } else {
            "workspace_invalid"
        }
    })?;
    let workspace_id = baseline
        .as_ref()
        .and_then(|value| value.workspace_id())
        .ok_or("workspace_not_initialized")?;
    let facts = root.join(".codeguard/findings");
    let tasks = root.join(".codeguard/tasks");
    if !real_directory(&facts) || !real_directory(&tasks) {
        return Err("workspace_records_unavailable");
    }
    let directory = facts.join(id);
    let projection_valid = match fs::symlink_metadata(tasks.join(format!("{id}.md"))) {
        Ok(metadata) => metadata.file_type().is_file(),
        Err(error) => allow_missing_projection && error.kind() == std::io::ErrorKind::NotFound,
    };
    if !real_directory(&directory) || !projection_valid {
        return Err("task_record_unavailable");
    }
    let fact: Value = serde_json::from_slice(&read_bounded(
        &directory.join("finding.json"),
        MAX_FACT_BYTES,
    )?)
    .map_err(|_| "finding_fact_invalid")?;
    if fact["id"] != id
        || fact["workspace_id"] != workspace_id
        || fact["state"] != "open"
        || fact["authority"] != "local_unverified"
        || fact["delivery_decision"] != "not_evaluated"
    {
        return Err("finding_fact_conflict");
    }
    Ok(candidate(root, id, &fact)?.brief)
}

fn build_view(
    root: &Path,
    checker_id: Option<&str>,
    task_ids: Option<&std::collections::BTreeSet<String>>,
) -> Result<Value, &'static str> {
    let baseline = read_workspace_baseline(root).map_err(|reason| {
        if reason == "legacy_workspace_requires_manual_migration" {
            "legacy_workspace_requires_manual_migration"
        } else {
            "workspace_invalid"
        }
    })?;
    let Some(baseline) = baseline else {
        return Ok(view(
            "actionable",
            "workspace_uninitialized",
            Value::Null,
            json!([["codeguard", "init", ".", "--apply"]]),
        ));
    };
    let workspace_id = baseline.workspace_id().ok_or("workspace_id_unavailable")?;
    let workspace = root.join(".codeguard");
    let findings = workspace.join("findings");
    let tasks = workspace.join("tasks");
    let reports = workspace.join("reports");
    let consumed = workspace.join("state/consumed");
    if ![&findings, &tasks, &reports]
        .iter()
        .all(|path| real_directory(path))
    {
        return Err("workspace_records_unavailable");
    }
    let report_queue = pending_reports(&reports, &consumed, workspace_id)?;
    if report_queue.untried {
        return Ok(view(
            "actionable",
            "pending_reports_require_sync",
            Value::Null,
            json!([["codeguard", "work", "sync", "."]]),
        ));
    }
    if !report_queue.failed.is_empty() {
        let mut response = view(
            "needs_decision",
            "failed_report_requires_repair",
            Value::Null,
            json!([]),
        );
        response["failed_reports"] = Value::Array(report_queue.failed);
        return Ok(response);
    }
    let mut candidates = Vec::new();
    for entry in fs::read_dir(&findings).map_err(|_| "findings_unreadable")? {
        let entry = entry.map_err(|_| "findings_unreadable")?;
        let path = entry.path();
        if !real_directory(&path) {
            return Err("finding_directory_invalid");
        }
        let id = entry
            .file_name()
            .into_string()
            .map_err(|_| "finding_id_invalid")?;
        if !safe_id(&id) {
            return Err("finding_id_invalid");
        }
        let fact: Value =
            serde_json::from_slice(&read_bounded(&path.join("finding.json"), MAX_FACT_BYTES)?)
                .map_err(|_| "finding_fact_invalid")?;
        if fact["id"] != id
            || fact["workspace_id"] != workspace_id
            || fact["state"] != "open"
            || fact["authority"] != "local_unverified"
            || fact["delivery_decision"] != "not_evaluated"
            || !fs::symlink_metadata(tasks.join(format!("{id}.md")))
                .is_ok_and(|metadata| metadata.file_type().is_file())
        {
            return Err("finding_fact_conflict");
        }
        let candidate = candidate(root, &id, &fact)?;
        if checker_id.is_none_or(|checker| candidate.brief["checker_id"] == checker)
            && task_ids.is_none_or(|ids| ids.contains(&candidate.id))
        {
            candidates.push(candidate);
        }
    }
    if candidates.is_empty() {
        let selected_command = if matches!(
            checker_id,
            Some("java.maven.p3c" | "java.maven.dependency_check")
        ) {
            "java"
        } else {
            "all"
        };
        let reason = if checker_id.is_some() {
            "no_selected_tasks_without_fresh_language_check"
        } else {
            "no_tasks_without_fresh_full_gate"
        };
        return Ok(view(
            "verification_required",
            reason,
            Value::Null,
            json!([["codeguard", "check", selected_command, "."]]),
        ));
    }
    candidates.sort_by(|left, right| {
        left.priority
            .cmp(&right.priority)
            .then(left.id.cmp(&right.id))
    });
    let selected_index = independent_source_task_index(root, &candidates).unwrap_or(0);
    let deferred_queries = if selected_index == 0 {
        json!([])
    } else {
        Value::Array(
            candidates
                .iter()
                .filter(|candidate| deferred_source_finding(candidate))
                .map(|candidate| json!(["codeguard", "task", "show", candidate.id, "."]))
                .collect(),
        )
    };
    let selected = candidates.remove(selected_index);
    let disposition = selected.brief["disposition"]
        .as_str()
        .ok_or("brief_disposition_invalid")?
        .to_owned();
    Ok(view(
        &disposition,
        "local_task_selected",
        selected.brief,
        deferred_queries,
    ))
}

// 仅在原首项是等待/待决策的源码问题时推进独立源码；完整依赖图未建立，
// 因此前置 blocker 或无法证明物理范围独立时保持原选择，不扩展修改授权。
fn deferred_source_finding(candidate: &Candidate) -> bool {
    candidate.brief["kind"] == "finding"
        && matches!(
            candidate.brief["disposition"].as_str(),
            Some("waiting" | "needs_decision")
        )
}

fn independent_source_task_index(root: &Path, candidates: &[Candidate]) -> Option<usize> {
    if !candidates.first().is_some_and(deferred_source_finding)
        || candidates
            .iter()
            .any(|candidate| candidate.brief["kind"] == "blocker")
    {
        return None;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        // 每个范围只读取一次；规范路径和 dev/ino 同时排除目录重叠、链接及硬链接别名。
        let scopes: Vec<_> = candidates
            .iter()
            .map(|candidate| {
                let scope = candidate.brief["scope"].as_str()?;
                let path = root.join(scope).canonicalize().ok()?;
                if !path.starts_with(root) {
                    return None;
                }
                let metadata = fs::metadata(&path).ok()?;
                metadata
                    .is_file()
                    .then_some((path, metadata.dev(), metadata.ino()))
            })
            .collect();
        candidates
            .iter()
            .enumerate()
            .skip(1)
            .find_map(|(index, candidate)| {
                if candidate.brief["kind"] != "finding"
                    || !matches!(
                        candidate.brief["disposition"].as_str(),
                        Some("actionable" | "verification_required")
                    )
                {
                    return None;
                }
                let source = scopes[index].as_ref()?;
                let independent = candidates
                    .iter()
                    .enumerate()
                    .filter(|(_, other)| deferred_source_finding(other))
                    .all(|(other_index, _)| {
                        scopes[other_index].as_ref().is_some_and(|other| {
                            !source.0.starts_with(&other.0)
                                && !other.0.starts_with(&source.0)
                                && (source.1, source.2) != (other.1, other.2)
                        })
                    });
                independent.then_some(index)
            })
    }
    #[cfg(not(unix))]
    {
        let _ = root;
        None
    }
}

fn candidate(root: &Path, id: &str, fact: &Value) -> Result<Candidate, &'static str> {
    let kind = fact["kind"].as_str().ok_or("finding_kind_invalid")?;
    let fingerprint = fact["fingerprint"]
        .as_str()
        .filter(|hash| valid_sha256(hash))
        .ok_or("finding_fingerprint_invalid")?;
    let expected_id = if kind == "blocker" {
        format!("CG-B-{}", &fingerprint[..32])
    } else if kind == "finding" {
        format!("CG-{}", &fingerprint[..32])
    } else {
        return Err("finding_kind_invalid");
    };
    let checker_id = fact["checker_id"]
        .as_str()
        .filter(|checker| {
            matches!(
                *checker,
                "syntax.native_confirmation"
                    | "node.eslint"
                    | "node.eslint.preparation"
                    | "node.npm.audit"
                    | "python.ruff"
                    | "python.ruff.doctor"
                    | "python.pip_audit"
                    | "go.vet"
                    | "shell.shellcheck"
                    | "rust.cargo_clippy"
                    | "rust.cargo_rustdoc"
                    | "rust.cargo_check"
                    | "rust.cargo_audit"
                    | "java.checkstyle"
                    | "java.checkstyle.preparation"
                    | "java.maven.p3c"
                    | "java.maven.dependency_check"
            )
        })
        .ok_or("finding_checker_invalid")?;
    if id != expected_id {
        return Err("finding_identity_invalid");
    }
    let first_run = fact["first_run_id"]
        .as_str()
        .filter(|id| safe_run_id(id))
        .ok_or("finding_run_invalid")?;
    let report_sha = fact["first_report_sha256"]
        .as_str()
        .filter(|sha| valid_sha256(sha))
        .ok_or("finding_report_invalid")?;
    let recheck = if checker_id == "syntax.native_confirmation" {
        json!(["codeguard", "task", "verify", id, ".", "--format", "json"])
    } else if matches!(checker_id, "node.eslint" | "node.eslint.preparation") {
        // 当前原生上下文失效时仍指向 ESLint；占位参数要求重新核验，不能猜测工具身份。
        json!([
            "codeguard",
            "lint",
            "typescript",
            root.join(
                fact["path"]
                    .as_str()
                    .or_else(|| fact["scope"].as_str())
                    .unwrap_or(".")
            ),
            "--workspace",
            root,
            "--node-tool",
            "<已核验绝对路径>",
            "--eslint-entry",
            "<已核验绝对路径>",
            "--eslint-version",
            "<已核验版本>",
            "--config",
            "<已核验原配置绝对路径>",
            "--cwd",
            "<已核验原工作目录绝对路径>"
        ])
    } else if checker_id == "shell.shellcheck" {
        // 原方言及显式rc由首次任务报告绑定，智能体不自行重建或替换规则上下文。
        json!([
            "codeguard",
            "task",
            "verify",
            id,
            root,
            "--shellcheck-tool",
            "<已核验的绝对路径>",
            "--format=json"
        ])
    } else if checker_id == "python.ruff.doctor" {
        json!([
            "codeguard",
            "doctor",
            ".",
            "--ruff-tool",
            "<已核验绝对路径>"
        ])
    } else if checker_id == "go.vet" {
        json!([
            "codeguard",
            "lint",
            "go",
            ".",
            "--go-tool",
            "<已核验绝对路径>"
        ])
    } else if matches!(checker_id, "rust.cargo_check" | "rust.cargo_rustdoc") {
        json!([
            "codeguard",
            "task",
            "verify",
            id,
            ".",
            "--cargo-tool",
            "<本轮已核验Cargo绝对路径>",
            "--format",
            "json"
        ])
    } else if checker_id == "rust.cargo_audit" {
        json!([
            "codeguard",
            "task",
            "verify",
            id,
            ".",
            "--cargo-audit-tool",
            "<本轮已核验绝对路径>",
            "--rustsec-db",
            "<已核验离线数据库绝对目录>",
            "--format",
            "json"
        ])
    } else if checker_id == "python.pip_audit" {
        json!([
            "codeguard",
            "task",
            "verify",
            id,
            ".",
            "--pip-audit-tool",
            "<本轮已核验绝对路径>",
            "--pip-audit-version",
            "<本轮已核验版本>",
            "--format",
            "json"
        ])
    } else if checker_id == "rust.cargo_clippy" {
        json!([
            "cargo",
            "clippy",
            "--locked",
            "--offline",
            "--all-targets",
            "--message-format=json"
        ])
    } else if checker_id == "node.npm.audit" {
        json!([
            "codeguard",
            "cve",
            "typescript",
            fact["build_root"],
            "--workspace",
            root,
            "--node-tool",
            "<原绝对路径>",
            "--npm-entry",
            "<原绝对路径>",
            "--npm-version",
            "<原具体版本>",
            "--userconfig",
            "<原绝对路径>",
            "--globalconfig",
            "<原绝对路径>",
            "--registry",
            "<已核验审计源>"
        ])
    } else if checker_id == "java.maven.dependency_check" {
        json!([
            "codeguard",
            "check",
            "java",
            ".",
            "--maven-tool",
            "<绝对路径>",
            "--java-home",
            "<绝对路径>",
            "--maven-repo",
            "<绝对路径>",
            "--repo-sha256",
            "<固定摘要>",
            "--cve-data-dir",
            "<离线漏洞库目录>",
            "--cve-data-sha256",
            "<固定摘要>"
        ])
    } else if checker_id == "java.checkstyle.preparation" {
        json!([
            "codeguard",
            "lint",
            "java",
            fact["first_affected_paths"][0],
            "--checker=checkstyle",
            "--workspace",
            ".",
            "--config",
            "<原配置>",
            "--java-tool",
            "<核对后的 Java>",
            "--checkstyle-jar",
            "<核对后的 JAR>"
        ])
    } else if checker_id == "java.checkstyle" {
        json!([
            "codeguard",
            "lint",
            "java",
            fact["path"],
            "--checker=checkstyle",
            "--workspace",
            ".",
            "--config",
            "<原配置绝对路径>",
            "--java-tool",
            "<原工具绝对路径>",
            "--checkstyle-jar",
            "<原 JAR 绝对路径>"
        ])
    } else if checker_id == "java.maven.p3c" {
        json!([
            "codeguard",
            "check",
            "all",
            ".",
            "--maven-tool",
            "<绝对路径>",
            "--java-home",
            "<绝对路径>",
            "--maven-repo",
            "<绝对路径>",
            "--repo-sha256",
            "<固定摘要>"
        ])
    } else {
        json!(["codeguard", "lint", "python", "."])
    };
    let mut brief = json!({
        "schema_version":if checker_id == "shell.shellcheck" {"0.17.0"} else if checker_id == "syntax.native_confirmation" {"0.3.0"} else {"0.1.0"}, "task_id":id, "kind":kind,
        "checker_id":checker_id, "evidence_ref":{
            "first_run_id":first_run, "first_report_sha256":report_sha
        },
        "recheck_argv":recheck,
        "history":"attempt_history_unavailable",
        "authority":"local_unverified",
        "closure_condition":"原生工具按相同受控策略完整复检并确认问题已解决；局部查询不能关闭任务"
    });
    let (priority, disposition) = if kind == "blocker" {
        let reason = fact["reason_code"]
            .as_str()
            .filter(|reason| safe_reason(reason))
            .ok_or("blocker_reason_invalid")?;
        let scope = fact["scope"]
            .as_str()
            .filter(|scope| *scope == "." || safe_relative_path(scope))
            .ok_or("blocker_scope_invalid")?;
        let build_root = fact["build_root"]
            .as_str()
            .filter(|scope| *scope == "." || safe_relative_path(scope))
            .ok_or("blocker_root_invalid")?;
        let paths = fact["first_affected_paths"]
            .as_array()
            .filter(|paths| {
                !paths.is_empty()
                    && paths
                        .iter()
                        .all(|path| path.as_str().is_some_and(safe_relative_path))
            })
            .ok_or("blocker_paths_invalid")?;
        if paths.iter().any(|path| {
            let value = path.as_str().expect("已校验路径");
            build_root != "." && !value.starts_with(&format!("{build_root}/"))
        }) || (scope != build_root && !paths.iter().any(|path| path == scope))
        {
            return Err("blocker_scope_conflict");
        }
        brief["reason_code"] = json!(reason);
        brief["scope"] = json!(scope);
        brief["build_root"] = json!(build_root);
        brief["affected_paths"] = json!(paths);
        brief["constraints"] = json!(["先恢复检查完整性", "不得关闭检查器或修改无关源码"]);
        let (priority, disposition, step) = if checker_id == "syntax.native_confirmation" {
            (
                1,
                "needs_decision",
                if fact["first_diagnostic_reason"] == "go_package_structure_candidate" {
                    "Go整文件候选未发现package声明；先恢复适用原生Go lint或编译器确认完整文件范围，核对声明应属于哪个包。注释或字符串不算声明；不得凭候选删除函数、猜包名或关闭任务，原生确认后只修复目标源码并复检"
                } else if fact["first_diagnostic_reason"] == "syntax_recovery_incomplete" {
                    "固定 grammar 的恢复扫描未完成或错误无法定位；恢复适用原生 lint/编译器，对原始源码确认。原生确认合法时调查 grammar 版本/兼容性或扫描预算；原生诊断成立时才按真实位置修复。缺 adapter 提出具体能力决策；原生确认前不得修改源码，不虚构错误位置，不凭零恢复关闭任务"
                } else {
                    "查看固定 grammar 与当前源码的疑似证据；通过同一任务的原生复检核对适用语法能力，工具或 adapter 缺失时提出具体恢复或能力决策，不能改用 Python 或凭 WASM 零恢复关闭任务"
                },
            )
        } else if checker_id == "python.ruff" && reason == "python_syntax_confirmation_needed" {
            (
                1,
                "actionable",
                "核对候选 Python 疑似位置，使用当前源码和适用原生语法能力复检；未完成前不按候选观察修改源码或关闭任务",
            )
        } else if checker_id == "python.ruff.doctor" {
            brief["first_diagnostic_reason"] = fact["first_diagnostic_reason"].clone();
            (
                1,
                "actionable",
                "查看本任务各次 doctor 报告与诊断，先恢复对应工具/版本或纠正选择；原生版本恢复后核验必需前置和批准来源，不能关闭检查器或修改无关源码",
            )
        } else if checker_id == "node.eslint.preparation" {
            brief["first_diagnostic_reason"] = fact["first_diagnostic_reason"].clone();
            (
                1,
                "actionable",
                "核对ESLint缺失上下文、版本与原配置；解析或抑制诊断先复现根因，恢复原检查后复扫，不修改无关源码",
            )
        } else if checker_id == "java.checkstyle.preparation" {
            brief["first_diagnostic_reason"] = fact["first_diagnostic_reason"].clone();
            (
                1,
                "actionable",
                "核对 Checkstyle 原工具和原配置的具体前置诊断，恢复后复扫；不修改无关源码",
            )
        } else if checker_id == "java.maven.p3c" && reason == "p3c_configuration_not_confirmed" {
            (
                0,
                "needs_decision",
                "确认该 Maven 模块是否要求 P3C；若要求，修正原生插件及规则配置并复扫；否则提交受保护策略决策",
            )
        } else if checker_id == "node.npm.audit" {
            (
                1,
                "actionable",
                "先恢复package.json及锁文件的可读普通文件和物理路径，再核对语法、重复字段、scripts类型，修正配置后恢复原生npm审计；再核验锁节点和漏洞源覆盖及时效，按原报告修订依赖并复检，局部零发现不得关闭任务",
            )
        } else if checker_id == "java.maven.dependency_check" {
            (
                1,
                "actionable",
                "核验 OWASP 原生检查、Maven 依赖归属与离线漏洞库时效；未经核验的 advisory 不作为已确认源码漏洞或白名单批准",
            )
        } else if checker_id == "rust.cargo_audit" {
            // 漏洞库时效需要独立核验，先推荐可直接修复的源码任务。
            (
                5,
                "actionable",
                "核对原生 cargo-audit advisory 与 Cargo.lock 解析版本，恢复工具或离线库；漏洞库时效、来源和完整策略未经核验前保持任务开放，误报仅提出待审候选",
            )
        } else if checker_id == "python.pip_audit" {
            (
                5,
                "actionable",
                "核对该 Python 构建根的标准锁、原生 pip-audit advisory 和解析版本；恢复工具或锁输入，验证漏洞源及时效。局部零漏洞与自写白名单均不能关闭任务",
            )
        } else if checker_id == "java.maven.p3c" {
            (
                1,
                "actionable",
                "恢复 Maven、JDK、固定离线依赖闭包或稳定报告，再运行原生 P3C 检查",
            )
        } else if checker_id == "go.vet" {
            (
                1,
                if reason == "go_policy_and_coverage_unverified" {
                    "needs_decision"
                } else {
                    "actionable"
                },
                "准备匹配的 Go 工具，核对模块配置和原生错误后复扫；规则与平台覆盖缺口须明确策略，不修改无关源码",
            )
        } else if checker_id == "rust.cargo_check" {
            (
                1,
                "actionable",
                "恢复原Cargo、锁定离线依赖和稳定输入；核验原生目标后执行build rust，类型检查不执行测试，不修改无关源码",
            )
        } else if checker_id == "rust.cargo_rustdoc" {
            (
                1,
                "actionable",
                "恢复原 Cargo、锁定离线依赖和稳定输入；歧义先核查原生目标，再运行 comments rust；不修改无关源码",
            )
        } else if checker_id == "rust.cargo_clippy" {
            (
                1,
                "actionable",
                if reason == "cargo_lock_unavailable" {
                    "先恢复项目原 Cargo.lock 或按项目依赖流程准备锁文件，再执行锁定离线 Clippy；不修改无关源码，不由检查器隐式生成锁"
                } else {
                    "恢复 Cargo Clippy 工具、配置或稳定输入，再运行锁定离线原生检查"
                },
            )
        } else if reason == "project_ruff_config_not_found" {
            (
                0,
                "needs_decision",
                "确认项目是否要求 Ruff；若要求则按批准策略配置，若不要求则修订策略",
            )
        } else if checker_id == "python.ruff" && reason == "ruff_local_tool_invalid" {
            (
                1,
                "actionable",
                "核对受检根 .venv/bin/ruff 的普通父目录、可执行入口和目标字节；恢复原本地环境后由同一入口复检，不删除环境绕过、不修改无关源码",
            )
        } else if reason.starts_with("ruff_tool_") || reason == "tool_identity_mismatch" {
            (
                1,
                "actionable",
                "检查 Ruff 可执行文件、版本和工具锁，准备匹配的原生工具",
            )
        } else {
            (
                1,
                "actionable",
                "检查原生工具、配置和输入稳定性，恢复完整检查",
            )
        };
        brief["step"] = json!(step);
        (priority, disposition)
    } else {
        let path = fact["path"]
            .as_str()
            .filter(|path| safe_relative_path(path))
            .ok_or("finding_path_invalid")?;
        let source_sha = fact["first_source_sha256"]
            .as_str()
            .filter(|sha| valid_sha256(sha))
            .ok_or("finding_source_invalid")?;
        let rule = fact["native_rule_id"]
            .as_str()
            .filter(|rule| {
                safe_rule(rule)
                    || (checker_id == "node.eslint"
                        && !rule.is_empty()
                        && rule.len() <= 512
                        && rule.bytes().all(|byte| {
                            byte.is_ascii_alphanumeric()
                                || matches!(byte, b'@' | b'/' | b'.' | b'_' | b'-' | b':')
                        }))
            })
            .ok_or("finding_rule_invalid")?;
        let same_source = read_bounded(&root.join(path), MAX_SOURCE_BYTES)
            .ok()
            .is_some_and(|bytes| format!("{:x}", Sha256::digest(bytes)) == source_sha);
        brief["scope"] = json!(path);
        brief["native_rule_id"] = json!(rule);
        brief["source_sha256"] = json!(source_sha);
        brief["constraints"] = json!(["仅修改目标源码", "不得忽略规则或将任务勾选当作复检"]);
        let step = finding_repair_step(rule);
        brief["step"] = json!(if same_source {
            step
        } else {
            "源码已变化；先重跑原生检查确认本问题仍存在"
        });
        (
            if same_source { 2 } else { 3 },
            if same_source {
                "actionable"
            } else {
                "verification_required"
            },
        )
    };
    if checker_id == "shell.shellcheck" {
        brief["step"] = json!(if kind == "blocker" {
            "核对原报告的ShellCheck版本、方言、source依赖和rc；先恢复检查能力，不修改无关源码。后续原生零诊断和配置抑制不能关闭此任务。"
        } else {
            "按当前原生SC规则和位置组修复目标源码，保持行为；源码变化后先重跑同方言同原配置检查，不关闭规则代替修复，按任务指引运行task verify；候选消失仍需可信政策和覆盖才能正式关闭。"
        });
    }
    brief["disposition"] = json!(disposition);
    let mut priority = priority;
    #[cfg(unix)]
    if checker_id == "shell.shellcheck" && kind == "finding" {
        let current = read_bounded(
            &root.join(format!(".codeguard/reports/{first_run}.json")),
            128 * 1024,
        )
        .ok()
        .filter(|b| format!("{:x}", Sha256::digest(b)) == report_sha)
        .and_then(|b| codeguard_adapters::parse_unique_json(&b).ok())
        .is_some_and(|r| crate::work_sync::shell_report::current(root, &r));
        if !current {
            brief["disposition"] = json!("verification_required");
            brief["step"] = json!(
                "Shell源码或原rc已经变化；先按同方言和原工具重新检查当前规则适用性，不按旧位置直接修改。配置抑制和零诊断不是已修复，任务正式关闭仍须复检流程。"
            );
            priority = 3;
        }
    }
    if checker_id == "shell.shellcheck" && kind == "blocker" {
        let reason = read_bounded(
            &root.join(format!(".codeguard/reports/{first_run}.json")),
            128 * 1024,
        )
        .ok()
        .filter(|b| format!("{:x}", Sha256::digest(b)) == report_sha)
        .and_then(|b| codeguard_adapters::parse_unique_json(&b).ok())
        .and_then(|r| r["native"]["reason"].as_str().map(str::to_owned));
        if matches!(
            reason.as_deref(),
            Some("shell_dialect_unresolved" | "shell_dialect_unsupported")
        ) {
            brief["disposition"] = json!("needs_decision");
            brief["step"] = json!(if reason.as_deref() == Some("shell_dialect_unresolved") {
                "原任务缺少已核验Shell方言；先确认shebang、文件约定或check --shell-dialect所代表的项目默认方言，再按确认的语境检查。不要猜成bash、重复安装或直接关闭旧任务。"
            } else {
                "原方言或文件声明不适用ShellCheck；确认实际方言和专用原生检查器，保留能力缺口。重复安装ShellCheck或强制改成bash不能恢复此检查。"
            });
            priority = 0;
        }
    }
    if checker_id == "rust.cargo_check" && kind == "finding" {
        brief["disposition"] = json!("verification_required");
        brief["step"] = json!(
            "按本轮原生编译错误和限定范围修复，使用同一Cargo运行task verify复检；类型检查不执行测试，后续仍须正式策略及覆盖核验，不自行关闭。"
        );
        priority = 3;
    }
    if checker_id == "rust.cargo_rustdoc" && kind == "finding" {
        brief["disposition"] = json!("verification_required");
        brief["step"] = json!(
            "使用同一 Cargo 执行 task verify 核对当前源码、清单、锁及原生范围；按本轮修复简报补齐文档或纠正链接。复检仍须策略核验，不能自行关闭。"
        );
        priority = 3;
    }
    #[cfg(unix)]
    if checker_id == "node.eslint.preparation" {
        let guidance = crate::eslint_preparation::guidance(root, fact);
        brief["disposition"] = guidance["disposition"].clone();
        brief["step"] = guidance["step"].clone();
        brief["preparation_guidance"] = guidance;
        priority = if brief["disposition"] == "actionable" {
            1
        } else {
            3
        };
    }
    // JavaScript 结构候选复用 ESLint 身份，但不能把非 ESLint 报告当作环境准备证据。
    if checker_id == "node.eslint.preparation"
        && fact["first_diagnostic_reason"] == "javascript_direct_binding_candidate"
    {
        brief["schema_version"] = json!("0.20.0");
        brief["disposition"] = json!("verification_required");
        brief["step"] = json!(
            "查看原报告 codeguard.javascript.duplicate_direct_lexical_binding 的位置和摘要；核对原 JavaScript 方言、构建根及 ESLint 配置，恢复适用原生 lint 确认后才修复并复检。候选仅覆盖顶层简单 let/const 名称，WASM 零恢复不能关闭任务"
        );
        brief["preparation_guidance"] =
            json!({"disposition":"verification_required", "step":brief["step"]});
        priority = 3;
    }
    if checker_id == "java.checkstyle.preparation" {
        let guidance = crate::checkstyle_preparation::guidance(
            root,
            id,
            fact["workspace_id"]
                .as_str()
                .ok_or("workspace_id_invalid")?,
        )?;
        brief["recheck_argv"] = guidance["recheck_argv"].clone();
        brief["affected_paths"] = guidance["affected_paths"].clone();
        brief["preparation_guidance"] = guidance.clone();
        brief["step"] = json!(if guidance["source_current"] == true {
            if guidance["diagnostic_reason"] == "checkstyle_configuration_regex_invalid" {
                "核对并修正 Checkstyle 原配置中的正则参数，再用同一原工具复扫；不要修改无关源码"
            } else {
                "恢复 Checkstyle 原工具/配置前置并按简报复扫；不要修改无关源码"
            }
        } else {
            "准备观察后的源文件已变化或不可用；先确认当前源码范围并复扫环境前置"
        });
        if !guidance["later_native_observation"].is_null() {
            brief["step"] = json!(
                "同一源码范围的原工具局部检查已运行；核对本次选择是否满足原前置、配置和批准覆盖后再决定关闭，勿重复恢复工具"
            );
            brief["disposition"] = json!("verification_required");
            priority = 4;
        } else if guidance["source_current"] != true {
            brief["disposition"] = json!("verification_required");
            priority = 0;
        }
    }
    if checker_id == "java.checkstyle" {
        let guidance = crate::checkstyle_workbench::task_guidance(
            root,
            id,
            fact["workspace_id"]
                .as_str()
                .ok_or("workspace_id_invalid")?,
        )?;
        brief["checkstyle_guidance"] = guidance.clone();
        brief["recheck_argv"] = guidance["recheck_argv"].clone();
        let source_current = read_bounded(
            &root.join(brief["scope"].as_str().ok_or("finding_path_invalid")?),
            MAX_SOURCE_BYTES,
        )
        .ok()
        .is_some_and(|bytes| guidance["source_sha256"] == format!("{:x}", Sha256::digest(bytes)));
        if guidance["configuration_current"] != true || !source_current {
            brief["disposition"] = json!("verification_required");
            brief["step"] = json!("源码或 Checkstyle 原配置已变化；先按原工具复扫，再决定修复方向");
            priority = 0;
        } else {
            brief["source_sha256"] = guidance["source_sha256"].clone();
            brief["disposition"] = json!("actionable");
            brief["step"] = guidance["repair_steps"][0].clone();
            priority = 2;
        }
    }
    let verification_observation = if checker_id == "syntax.native_confirmation" {
        None
    } else {
        latest_verification_observation(root, id, fact, &brief)?
    };
    if let Some(observation) = verification_observation.as_ref() {
        let outcome = observation.outcome.as_str();
        brief["verification_run_id"] = json!(observation.run_id);
        if let Some(reason) = observation.checkstyle_reason.as_ref() {
            brief["verification_reason"] = json!(reason);
        }
        brief["verification_report_sha256"] = json!(observation.report_sha256);
        let observed_source_matches = observation.source_sha256.as_ref().is_some_and(|sha| {
            (if checker_id == "java.checkstyle.preparation" {
                observation.checkstyle_source_path.as_deref()
            } else {
                brief["scope"].as_str()
            })
            .is_some_and(|path| {
                read_bounded(&root.join(path), MAX_SOURCE_BYTES)
                    .ok()
                    .is_some_and(|bytes| format!("{:x}", Sha256::digest(bytes)) == *sha)
            })
        });
        let checkstyle_config_changed =
            matches!(
                checker_id,
                "java.checkstyle" | "java.checkstyle.preparation"
            ) && observation.config_sha256.as_ref().is_some_and(|sha| {
                !observation
                    .checkstyle_config_path
                    .as_ref()
                    .is_some_and(|path| {
                        read_bounded(Path::new(path), 1024 * 1024)
                            .ok()
                            .is_some_and(|bytes| format!("{:x}", Sha256::digest(bytes)) == *sha)
                    })
            });
        let checkstyle_tools_changed =
            observation
                .checkstyle_tool_inputs
                .as_ref()
                .is_some_and(|inputs| {
                    ["java", "jar"].iter().any(|key| {
                        let input = &inputs[*key];
                        !input["path"].as_str().is_some_and(|path| {
                            let path = Path::new(path);
                            path.is_absolute()
                                && path.canonicalize().ok().as_deref() == Some(path)
                                && codeguard_runtime::read_bounded_regular_file(
                                    path,
                                    128 * 1024 * 1024,
                                )
                                .ok()
                                .is_some_and(|bytes| {
                                    input["sha256"] == format!("{:x}", Sha256::digest(bytes))
                                })
                        })
                    })
                });
        let cve_config_changed = checker_id == "java.maven.dependency_check"
            && observation.config_sha256.as_ref().is_some_and(|sha| {
                !brief["affected_paths"][0].as_str().is_some_and(|path| {
                    read_bounded(&root.join(path), 4 * 1024 * 1024)
                        .ok()
                        .is_some_and(|bytes| format!("{:x}", Sha256::digest(bytes)) == *sha)
                })
            });
        if checkstyle_tools_changed {
            brief["verification_invalidated_reason"] = json!("tool_inputs_changed_or_unavailable");
            brief["disposition"] = json!("verification_required");
            brief["step"] = json!(
                "复检后的 Java 或 Checkstyle 工具已变化或不可用；核对当前工具身份并重新复检，不沿用旧恢复或缺失结论"
            );
            priority = 0;
        } else if checkstyle_config_changed {
            brief["verification_invalidated_reason"] =
                json!("configuration_input_changed_or_unavailable");
            brief["disposition"] = json!("verification_required");
            brief["step"] = json!("复检后 Checkstyle 配置再次变化；重新运行原工具核对原规则和覆盖");
            priority = 0;
        } else if matches!(
            checker_id,
            "java.checkstyle" | "java.checkstyle.preparation"
        ) && observation.source_sha256.is_some()
            && !observed_source_matches
        {
            brief["verification_invalidated_reason"] = json!("source_input_changed_or_unavailable");
            brief["disposition"] = json!("verification_required");
            brief["step"] = json!(
                "复检后的 Checkstyle 目标源码已变化或不可用；重新确认当前范围并运行原工具，不沿用旧诊断修复方向"
            );
            priority = 0;
        } else if checker_id == "python.ruff"
            && brief["reason_code"] == "python_syntax_confirmation_needed"
            && observation.source_sha256.is_some()
            && (!observed_source_matches
                || observation
                    .ruff_configuration
                    .as_ref()
                    .is_some_and(|(config, sha)| {
                        !brief["scope"].as_str().is_some_and(|path| {
                            crate::ruff_verification_configuration::is_current(
                                root, path, config, sha,
                            )
                        })
                    }))
        {
            brief["verification_invalidated_reason"] = json!(if !observed_source_matches {
                "source_input_changed_or_unavailable"
            } else {
                "configuration_input_changed_or_unavailable"
            });
            brief["disposition"] = json!("verification_required");
            brief["step"] = json!(
                "Python确认复检后的源码或配置已变化；按同一任务范围重新运行原生Ruff，不沿用旧修复或恢复判断"
            );
            priority = 0;
        } else if checker_id == "python.ruff"
            && brief["reason_code"] == "python_syntax_confirmation_needed"
            && outcome == "still_present"
            && observed_source_matches
        {
            brief["verification_observation"] = json!(outcome);
            brief["disposition"] = json!("actionable");
            brief["step"] = json!(
                "本轮Ruff已确认该文件存在原生语法错误；按本任务报告的原生位置核对并修复源码，保持预期语义，不用空实现逃避检查；修复后对同一任务复检，不能凭WASM或任务勾选关闭"
            );
            brief["constraints"] = json!([
                "仅修改本任务绑定的Python文件",
                "修复前核对本轮原生位置与当前源码",
                "不得关闭原生检查器或用空实现消除错误"
            ]);
            priority = 1;
        } else if matches!(
            outcome,
            "candidate_absent_unverified_policy" | "environment_restored_unverified_policy"
        ) {
            if cve_config_changed {
                brief["disposition"] = json!("verification_required");
                brief["step"] =
                    json!("复检后 Maven 配置已变化；重新运行原生 CVE 检查，再核验漏洞库和依赖归属");
                priority = 0;
            } else if kind == "finding" && !observed_source_matches {
                if checker_id == "java.checkstyle" {
                    brief["verification_invalidated_reason"] =
                        json!("source_input_changed_or_unavailable");
                }
                brief["disposition"] = json!("verification_required");
                brief["step"] = json!("复检后源码再次变化；重新运行原生检查确认当前问题状态");
                priority = 0;
            } else {
                brief["verification_observation"] = json!(outcome);
                if kind == "finding" {
                    brief["source_sha256"] = json!(observation.source_sha256);
                }
                brief["disposition"] = json!("verification_required");
                brief["step"] = json!(if checker_id == "python.ruff.doctor" {
                    "原生版本诊断已恢复；仅版本观察，核验批准前置与工具锁并执行原受阻质量检查后再决定关闭"
                } else if kind == "blocker" {
                    "原受阻义务已在本地复检中运行；核验受保护工具和规则策略后再决定关闭"
                } else {
                    "原生复检未再检出该问题；核验规则仍启用、覆盖和批准策略后再决定关闭"
                });
                priority = 4;
            }
        } else if outcome == "suppression_requires_review" {
            brief["verification_observation"] = json!(outcome);
            brief["disposition"] = json!(if kind == "finding" && !observed_source_matches {
                "verification_required"
            } else {
                "needs_decision"
            });
            brief["step"] = json!(if kind == "finding" && !observed_source_matches {
                "复检后源码再次变化；重新运行原生检查和抑制对照"
            } else if checker_id == "rust.cargo_clippy" {
                "原生 Clippy force-warn 对照重新检出原规则；核查源码 allow、Cargo lints 或其它抑制，按批准策略处理后复检"
            } else {
                "原生对照发现同规则的源码注释抑制；核查它是否遮蔽原问题，按批准策略移除或裁定后重新运行原工具"
            });
            priority = 0;
        } else if outcome == "rule_coverage_requires_review" {
            brief["verification_observation"] = json!(outcome);
            brief["disposition"] = json!(if checker_id == "go.vet" && !observed_source_matches {
                "verification_required"
            } else {
                "needs_decision"
            });
            brief["step"] = json!(if checker_id == "go.vet" && !observed_source_matches {
                "Go 源码在覆盖复核后变化；重新运行原生检查确认当前构建选择与问题状态"
            } else if checker_id == "go.vet" {
                if observation.go_task_scope.as_ref().is_some_and(|scope| {
                    matches!(scope["status"].as_str(), Some("excluded" | "not_selected"))
                }) {
                    "原任务文件未被本轮默认构建条件选中；核查构建标签、平台与 CGO 配置后复扫，不把漏扫当修复"
                } else {
                    "同文件同规则仍有不同身份的原生诊断；核对原问题是否仅位置或源码锚点变化，不凭指纹变化关闭任务"
                }
            } else if checker_id == "rust.cargo_clippy" {
                "Clippy 本轮未再报告原问题；核查 allow/cap-lints、Cargo lints、特性组合和工具身份后重新复检"
            } else if checker_id == "java.maven.p3c" {
                "P3C 命名规则局部复检未再报告原问题；核查原生报告的文件覆盖、配置及工具身份，完成全规则与策略复检后再裁定"
            } else {
                "原生生效设置未启用原规则；恢复受批准的规则覆盖并重新复检，或提交独立策略决策"
            });
            priority = 0;
        } else if checker_id == "node.npm.audit"
            && matches!(outcome, "still_blocked" | "incomplete")
        {
            brief["verification_observation"] = json!(outcome);
            brief["disposition"] = json!("actionable");
            brief["step"] = json!(
                "npm原工具复检已记录；核对本轮诊断、漏洞源覆盖及时效，零advisory不代表完整性任务已解决，不修改无关源码或自动加入白名单"
            );
        } else if checker_id == "rust.cargo_audit"
            && matches!(outcome, "still_blocked" | "incomplete")
        {
            brief["verification_observation"] = json!(outcome);
            brief["disposition"] = json!("actionable");
            brief["step"] = json!(
                "cargo-audit 原工具复检已记录；核对本轮诊断、锁文件中的依赖归属及漏洞库来源和时效。覆盖未获核验时保持任务开放，不把 advisory 直接加入误报白名单"
            );
        } else if checker_id == "python.pip_audit"
            && matches!(outcome, "still_blocked" | "incomplete")
        {
            brief["verification_observation"] = json!(outcome);
            brief["disposition"] = json!("actionable");
            brief["step"] = json!(
                "pip-audit 原工具复检已记录；核对标准锁的环境、依赖组、原生 advisory 与漏洞源时效。局部零漏洞不能关闭任务，误报仅提出待审候选"
            );
        } else if checker_id == "java.checkstyle.preparation"
            && matches!(outcome, "still_blocked" | "incomplete")
        {
            brief["verification_observation"] = json!(outcome);
            // 已完成的失败复检应允许继续恢复前置；输入变化仍先重新确认范围。
            let source_current = brief["preparation_guidance"]["source_current"] == true;
            if !source_current {
                brief["verification_invalidated_reason"] =
                    json!("source_input_changed_or_unavailable");
            }
            brief["disposition"] = json!(if source_current {
                "actionable"
            } else {
                "verification_required"
            });
            brief["step"] = json!(if source_current {
                if brief["preparation_guidance"]["diagnostic_reason"]
                    == "checkstyle_configuration_regex_invalid"
                {
                    "Checkstyle 原配置正则仍无效；核对并修正正则参数后复扫，不修改无关源码"
                } else {
                    "Checkstyle 前置复检仍受阻或未完成；根据具体诊断恢复原工具、配置或报告，不修改无关源码"
                }
            } else {
                "准备观察后的源码已变化或不可用；先重新确认当前范围再恢复原工具前置"
            });
            priority = 0;
        } else if checker_id == "java.checkstyle" && outcome == "incomplete" {
            brief["verification_observation"] = json!(outcome);
            brief["disposition"] = json!("verification_required");
            brief["step"] = json!(
                "Checkstyle 原工具复检未完成；核对保存报告中的具体工具、配置或输入原因后复检，不修改无关源码"
            );
            priority = 0;
        } else if checker_id == "go.vet" && matches!(outcome, "still_blocked" | "incomplete") {
            brief["verification_observation"] = json!(outcome);
            brief["step"] = json!(
                "Go 原工具复检仍受阻或未完成；核对本轮报告中的工具、模块和编译问题后复检，不修改无关源码"
            );
            priority = 0;
        } else if outcome == "still_present" && kind == "finding" && observed_source_matches {
            brief["source_sha256"] = json!(observation.source_sha256);
            brief["verification_observation"] = json!(outcome);
            brief["disposition"] = json!("actionable");
            brief["step"] = if checker_id == "java.checkstyle" {
                brief["checkstyle_guidance"]["repair_steps"][0].clone()
            } else {
                json!(finding_repair_step(
                    brief["native_rule_id"]
                        .as_str()
                        .ok_or("finding_rule_invalid")?
                ))
            };
            priority = 2;
        }
    }
    if verification_observation
        .as_ref()
        .and_then(|observation| observation.go_module_identities.as_ref())
        .is_some_and(|identities| {
            !crate::task_verify_command::go_task_inputs_stable(
                root,
                &json!({"module_identities":identities}),
            )
        })
    {
        brief["disposition"] = json!("verification_required");
        brief["step"] =
            json!("Go 模块配置在复检后变化；重新运行原工具，不沿用旧的未发现或环境恢复观察");
        priority = 0;
    }
    if verification_observation
        .as_ref()
        .and_then(|observation| observation.go_source_snapshot_sha256.as_deref())
        .is_some_and(|expected| {
            crate::go_lint_command::current_source_snapshot_sha256(root).as_deref()
                != Some(expected)
        })
    {
        brief["disposition"] = json!("verification_required");
        brief["step"] = json!("Go 源码输入在复检后变化；重新运行原工具，不沿用旧观察");
        priority = 0;
    }
    if checker_id == "python.ruff"
        && kind == "finding"
        && verification_observation
            .as_ref()
            .is_some_and(|observation| {
                matches!(
                    observation.outcome.as_str(),
                    "still_present"
                        | "candidate_absent_unverified_policy"
                        | "suppression_requires_review"
                        | "rule_coverage_requires_review"
                ) && !observation
                    .ruff_configuration
                    .as_ref()
                    .is_some_and(|(path, sha)| {
                        brief["scope"].as_str().is_some_and(|source| {
                            crate::ruff_verification_configuration::is_current(
                                root, source, path, sha,
                            )
                        })
                    })
            })
    {
        brief
            .as_object_mut()
            .expect("任务简报是对象")
            .remove("verification_observation");
        brief["disposition"] = json!("verification_required");
        brief["step"] = json!(
            "Ruff 配置内容或选择优先级在复检后变化，或旧报告缺配置身份；重新运行原工具，不沿用旧纠错提案"
        );
        priority = 0;
    }
    #[cfg(unix)]
    if checker_id == "node.eslint" {
        let guidance = crate::eslint_workbench::guidance(root, fact);
        brief["disposition"] = guidance["disposition"].clone();
        brief["step"] = guidance["step"].clone();
        if guidance["recheck_argv"].is_array() {
            brief["recheck_argv"] = guidance["recheck_argv"].clone();
        }
        brief["eslint_guidance"] = guidance;
        priority = if brief["disposition"] == "actionable" {
            2
        } else {
            0
        };
    }
    if checker_id == "syntax.native_confirmation" {
        if let Some(guidance) = crate::syntax_task_recheck::guidance(root, &brief) {
            if matches!(guidance["schema_version"].as_str(), Some("0.7.0" | "0.9.0")) {
                brief["schema_version"] = guidance["schema_version"].clone();
                brief["native_adapter"] = guidance["native_adapter"].clone();
                brief["tool_readiness"] = guidance["tool_readiness"].clone();
            } else if matches!(
                guidance["schema_version"].as_str(),
                Some(
                    "0.4.0"
                        | "0.5.0"
                        | "0.6.0"
                        | "0.8.0"
                        | "0.10.0"
                        | "0.11.0"
                        | "0.12.0"
                        | "0.14.0"
                        | "0.15.0"
                        | "0.18.0"
                        | "0.19.0"
                )
            ) {
                brief["schema_version"] = guidance["schema_version"].clone();
                brief["native_confirmation_reason"] =
                    guidance["native_confirmation_reason"].clone();
                brief["native_column_unit"] = guidance["native_column_unit"].clone();
            }
            if matches!(
                guidance["schema_version"].as_str(),
                Some("0.14.0" | "0.15.0" | "0.18.0" | "0.19.0")
            ) {
                brief["native_confirmation_ref"] = guidance["native_confirmation_ref"].clone();
            }
            brief["disposition"] = guidance["disposition"].clone();
            brief["step"] = guidance["step"].clone();
            for key in [
                "native_confirmation_status",
                "native_confirmation_ref",
                "native_diagnostic_positions",
                "native_context_diagnostics",
            ] {
                if !guidance[key].is_null() {
                    brief[key] = guidance[key].clone();
                }
            }
            if guidance["recheck_argv"].is_array() {
                brief["recheck_argv"] = guidance["recheck_argv"].clone();
            }
            priority = if brief["disposition"] == "actionable" {
                2
            } else {
                0
            };
        }
    }
    #[cfg(unix)]
    if checker_id == "syntax.native_confirmation"
        || (checker_id == "python.ruff"
            && brief["reason_code"] == "python_syntax_confirmation_needed"
            && !brief["verification_invalidated_reason"].is_string())
    {
        if let Some(guidance) = crate::task_lifecycle_store::guidance(
            root,
            &brief,
            fact["workspace_id"]
                .as_str()
                .ok_or("workspace_identity_unavailable")?,
        ) {
            brief["disposition"] = guidance["disposition"].clone();
            brief["step"] = guidance["step"].clone();
            if checker_id == "syntax.native_confirmation" {
                brief["native_diagnostic_positions"] = json!([]);
            }
            priority = 0;
        }
    }
    let action_id = canonical_action_id(&brief)?;
    brief["action_id"] = json!(action_id);
    #[cfg(unix)]
    {
        let history = crate::task_attempt_command::attempt_history(root, id, &brief)?;
        if history["open_attempt_id"].is_string() {
            brief["disposition"] = json!("waiting");
            brief["step"] =
                json!("现有修复尝试尚未结束；由持有租约的执行者记录结果，或待过期接管恢复");
            priority = 0;
        } else if history["awaiting_verification"] == true {
            brief["disposition"] = json!("verification_required");
            brief["step"] = json!(
                "修复动作已结束；运行 task verify 取得原检查器复检结果，再决定后续修复或关闭条件"
            );
            priority = 0;
        } else if history["no_progress_count"].as_u64().unwrap_or(0) >= 2
            && brief["disposition"] == "actionable"
        {
            brief["disposition"] = json!("needs_decision");
            brief["step"] = json!(
                "同一输入与动作连续无进展；检查原生诊断、工具配置和失败记录，提出具体修复或策略决策"
            );
            priority = 0;
        }
        brief["history"] = history;
    }
    if kind == "finding" && brief["disposition"] != "waiting" {
        let (proposals, projections) = current_correction_refs(root, id, &brief)?;
        if !proposals.is_empty() {
            brief["correction_proposal_refs"] = json!(proposals);
            if !projections.is_empty() {
                brief["correction_task_refs"] = json!(projections);
            }
            brief["disposition"] = json!("needs_decision");
            brief["step"] = json!(
                "已有同一问题的白名单纠错待审提案；核对原工具复检、旧决策和精确范围，提交独立策略评审；提案本身不能放行"
            );
            priority = 0;
        }
    }
    Ok(Candidate {
        id: id.into(),
        priority,
        brief,
    })
}

fn current_correction_refs(
    root: &Path,
    id: &str,
    brief: &Value,
) -> Result<(Vec<String>, Vec<String>), &'static str> {
    let Some(run_id) = brief["verification_run_id"].as_str() else {
        return Ok((Vec::new(), Vec::new()));
    };
    let Some(report_sha) = brief["verification_report_sha256"].as_str() else {
        return Ok((Vec::new(), Vec::new()));
    };
    let events = root.join(".codeguard/findings").join(id).join("events");
    if !real_directory(&events) {
        return Err("task_events_unavailable");
    }
    let mut references = Vec::new();
    let mut projections = Vec::new();
    for entry in fs::read_dir(events).map_err(|_| "task_events_unreadable")? {
        let entry = entry.map_err(|_| "task_events_unreadable")?;
        let name = entry.file_name();
        let Some(name) = name.to_str() else { continue };
        let Some(digest) = name
            .strip_prefix("correction-proposed-")
            .and_then(|part| part.strip_suffix(".json"))
        else {
            continue;
        };
        if !valid_sha256(digest) {
            continue;
        }
        let Ok(bytes) = read_bounded(&entry.path(), MAX_FACT_BYTES) else {
            continue;
        };
        if format!("{:x}", Sha256::digest(&bytes)) != digest {
            continue;
        }
        let Ok(event) = serde_json::from_slice::<Value>(&bytes) else {
            continue;
        };
        if event["schema_version"] != "0.1.0"
            || event["event_type"] != "whitelist_correction_proposed"
            || event["task_id"] != id
            || event["finding_id"] != id
            || event["authority"] != "unverified"
            || event["gate_effect"] != "none"
            || event["proposal"]["finding_id"] != id
            || event["proposal"]["verification_ref"]["run_id"] != run_id
            || event["proposal"]["verification_ref"]["report_sha256"] != report_sha
            || event["proposal"]["verification_ref"]["observation"]
                != brief["verification_observation"]
        {
            continue;
        }
        let reference = format!(".codeguard/findings/{id}/events/{name}");
        let projection_ref = format!(".codeguard/tasks/corrections/{id}/{digest}.md");
        // 附件可写；只展示与当前事件重建字节完全相同的投影，不从附件读指令。
        let corrections = root.join(".codeguard/tasks/corrections");
        if real_directory(&root.join(".codeguard/tasks"))
            && real_directory(&corrections)
            && real_directory(&corrections.join(id))
            && read_bounded(&root.join(&projection_ref), MAX_FACT_BYTES).is_ok_and(|bytes| {
                bytes
                    == crate::whitelist_correction_projection::render(&event, &reference, brief)
                        .as_bytes()
            })
        {
            projections.push(projection_ref);
        }
        references.push(reference);
    }
    references.sort();
    projections.sort();
    Ok((references, projections))
}

fn finding_repair_step(rule: &str) -> &'static str {
    match rule {
        "F401" => "核对导入是否仍被使用；确认后仅修改目标文件导入",
        "E501" => "核对已配置行长，保持语义并重排行内容",
        _ => "查阅原生规则与私有诊断，先确认根因再修复",
    }
}

/// 将结构化问题类型映射到受控动作，避免执行者改名重置尝试预算。
pub(crate) fn canonical_action_id(brief: &Value) -> Result<&'static str, &'static str> {
    match brief["kind"].as_str() {
        Some("finding") => Ok("repair-source"),
        Some("blocker")
            if brief["checker_id"] == "python.ruff"
                && brief["reason_code"] == "python_syntax_confirmation_needed"
                && brief["verification_observation"] == "still_present"
                && !brief["verification_invalidated_reason"].is_string() =>
        {
            Ok("repair-source")
        }

        Some("blocker")
            if brief["checker_id"] == "syntax.native_confirmation"
                && (brief["native_confirmation_status"] == "diagnostics_observed"
                    || (matches!(brief["schema_version"].as_str(), Some("0.8.0" | "0.10.0"))
                        && brief["native_diagnostic_positions"]
                            .as_array()
                            .is_some_and(|r| !r.is_empty()))) =>
        {
            Ok("repair-source")
        }
        Some("blocker")
            if brief["reason_code"] == "project_ruff_config_not_found"
                || brief["reason_code"] == "p3c_configuration_not_confirmed" =>
        {
            Ok("review-project-policy")
        }
        Some("blocker") => Ok("restore-checker-environment"),
        _ => Err("attempt_kind_invalid"),
    }
}

fn latest_verification_observation(
    root: &Path,
    id: &str,
    fact: &Value,
    brief: &Value,
) -> Result<Option<VerificationObservation>, &'static str> {
    let events = root.join(format!(".codeguard/findings/{id}/events"));
    if !real_directory(&events) {
        return Err("task_events_unavailable");
    }
    let mut latest_run = 0_u128;
    let mut latest_verify = None;
    for entry in fs::read_dir(events).map_err(|_| "task_events_unreadable")? {
        let entry = entry.map_err(|_| "task_events_unreadable")?;
        let path = entry.path();
        if path.extension().is_none_or(|extension| extension != "json") {
            continue;
        }
        let stem = path
            .file_stem()
            .and_then(|value| value.to_str())
            .ok_or("task_event_name_invalid")?;
        let is_verify = stem.starts_with("verify-");
        let run_id = stem.strip_prefix("verify-").unwrap_or(stem);
        let Some(sequence) = run_sequence(run_id) else {
            continue;
        };
        if sequence < latest_run {
            continue;
        }
        if is_verify {
            let event: Value = serde_json::from_slice(&read_bounded(&path, 4096)?)
                .map_err(|_| "verification_event_invalid")?;
            if !matches!(
                event["schema_version"].as_str(),
                Some("0.1.0" | "0.2.0" | "0.3.0")
            ) || event["event"] != "verification_observed"
                || event["task_id"] != id
                || event["kind"] != fact["kind"]
                || event["workspace_id"] != fact["workspace_id"]
                || event["run_id"] != run_id
                || event["state_after"] != "open"
                || event["authority"] != "local_unverified"
            {
                return Err(if brief["checker_id"] == "rust.cargo_audit" {
                    "rust_cve_verification_event_header_invalid"
                } else {
                    "verification_event_invalid"
                });
            }
            let report_path = root.join(format!(".codeguard/reports/{run_id}.json"));
            let report_bytes = match read_bounded(&report_path, 16 * 1024 * 1024) {
                Ok(bytes) => bytes,
                Err(_) if !report_path.exists() => {
                    // reports 默认不入 Git；跨机器缺本地报告时不能沿用恢复候选。
                    latest_verify = None;
                    latest_run = sequence;
                    continue;
                }
                Err(reason) => return Err(reason),
            };
            let report: Value =
                serde_json::from_slice(&report_bytes).map_err(|_| "verification_event_invalid")?;
            if brief["checker_id"] == "node.npm.audit" {
                codeguard_adapters::parse_unique_json(&report_bytes)
                    .map_err(|_| "verification_event_invalid")?;
                if !crate::npm_task_recheck::matches_task(brief, &report) {
                    return Err("verification_event_invalid");
                }
            }
            // 输入已变化的npm历史复检不能继续解释当前依赖，也不阻断新的复检指引。
            if brief["checker_id"] == "rust.cargo_check"
                && event["report_sha256"] == format!("{:x}", Sha256::digest(&report_bytes))
                && !crate::rust_build_task_recheck::inputs_current(root, &report)
            {
                latest_verify = None;
                latest_run = sequence;
                continue;
            }
            if brief["checker_id"] == "rust.cargo_audit"
                && event["report_sha256"] == format!("{:x}", Sha256::digest(&report_bytes))
                && !crate::rust_cve_task_recheck::inputs_current(root, &report)
            {
                latest_verify = None;
                latest_run = sequence;
                continue;
            }
            if brief["checker_id"] == "python.pip_audit"
                && event["report_sha256"] == format!("{:x}", Sha256::digest(&report_bytes))
                && !crate::python_cve_task_recheck::inputs_current(root, &report)
            {
                latest_verify = None;
                latest_run = sequence;
                continue;
            }
            if brief["checker_id"] == "shell.shellcheck"
                && event["report_sha256"] == format!("{:x}", Sha256::digest(&report_bytes))
                && !crate::shell_task_recheck::inputs_current(root, &report)
            {
                latest_verify = None;
                latest_run = sequence;
                continue;
            }
            if brief["checker_id"] == "rust.cargo_rustdoc"
                && event["report_sha256"] == format!("{:x}", Sha256::digest(&report_bytes))
                && !crate::rustdoc_task_recheck::inputs_current(root, &report)
            {
                latest_verify = None;
                latest_run = sequence;
                continue;
            }
            if brief["checker_id"] == "node.npm.audit"
                && event["report_sha256"] == format!("{:x}", Sha256::digest(&report_bytes))
                && crate::npm_task_recheck::inputs_stale(root, &report)
            {
                latest_verify = None;
                latest_run = sequence;
                continue;
            }
            let python_scoped_report = brief["checker_id"] == "python.ruff"
                && matches!(report["schema_version"].as_str(), Some("0.18.0" | "0.19.0"));
            let go_report = brief["checker_id"] == "go.vet";
            let rust_report = brief["checker_id"] == "rust.cargo_clippy";
            let java_report = brief["checker_id"] == "java.maven.p3c";
            let checkstyle_report = brief["checker_id"] == "java.checkstyle";
            let preparation_report = brief["checker_id"] == "java.checkstyle.preparation";
            let cve_report = brief["checker_id"] == "java.maven.dependency_check";
            let report_shape_valid = if brief["checker_id"] == "python.ruff.doctor" {
                crate::work_sync::valid_doctor_report(&report)
                    && event["observation"] == classify_doctor(brief, &report)
            } else if matches!(
                brief["checker_id"].as_str(),
                Some("node.eslint" | "node.eslint.preparation")
            ) {
                crate::eslint_task_recheck::valid_shape(&report)
                    && event["observation"] == crate::eslint_task_recheck::classify(brief, &report)
            } else if brief["checker_id"] == "node.npm.audit" {
                crate::work_sync::valid_npm_observation(root, &report)
                    && event["observation"]
                        == crate::npm_task_recheck::classify(root, brief, &report)
            } else if go_report {
                report["report_type"] == "go_lint_local_feedback"
                    && matches!(
                        report["schema_version"].as_str(),
                        Some("0.4.0" | "0.5.0" | "0.6.0" | "0.7.0")
                    )
                    && report["checker_id"] == "go.vet"
                    && event["observation"] == classify_go(brief, &report)
            } else if preparation_report {
                crate::checkstyle_preparation_recheck::valid_shape(&report)
                    && event["observation"]
                        == crate::checkstyle_preparation_recheck::classify(brief, &report)
            } else if checkstyle_report {
                crate::checkstyle_task_recheck::valid_shape(&report)
                    && event["observation"]
                        == crate::checkstyle_task_recheck::classify(brief, &report)
            } else if java_report {
                report["report_type"] == "java_p3c_project_observation"
                    && report["schema_version"] == "0.2.0"
                    && report["checker_id"] == "java.maven.p3c"
                    && report["coverage_proven"] == false
                    && event["observation"] == classify_java(brief, &report)
            } else if cve_report {
                report["report_type"] == "java_cve_project_probe"
                    && report["schema_version"] == "0.3.0"
                    && report["checker_id"] == "java.maven.dependency_check"
                    && report["coverage_proven"] == false
                    && event["observation"] == classify_cve(brief, &report)
            } else if brief["checker_id"] == "rust.cargo_check" {
                crate::rust_build_task_recheck::valid_shape(&report)
                    && event["observation"]
                        == crate::rust_build_task_recheck::classify(brief, &report)
            } else if brief["checker_id"] == "rust.cargo_audit" {
                crate::rust_cve_task_recheck::valid_shape(&report)
                    && event["observation"]
                        == crate::rust_cve_task_recheck::classify(brief, &report)
            } else if brief["checker_id"] == "python.pip_audit" {
                crate::work_sync::valid_python_cve_observation(root, &report)
                    && event["observation"]
                        == crate::python_cve_task_recheck::classify(brief, &report)
            } else if brief["checker_id"] == "shell.shellcheck" {
                crate::shell_task_recheck::valid_shape(root, &report)
                    && event["observation"] == crate::shell_task_recheck::classify(brief, &report)
            } else if brief["checker_id"] == "rust.cargo_rustdoc" {
                crate::rustdoc_task_recheck::valid_shape(&report)
                    && event["observation"] == crate::rustdoc_task_recheck::classify(brief, &report)
            } else if rust_report {
                report["report_type"] == "rust_clippy_local_observation"
                    && matches!(report["schema_version"].as_str(), Some("0.2.0" | "0.3.0"))
                    && report["checker_id"] == "rust.cargo_clippy"
                    && report["coverage_proven"] == false
                    && event["observation"] == classify_rust(brief, &report)
            } else {
                report["report_type"] == "python_lint_feedback"
                    && (report["schema_version"] == "0.4.0"
                        || report["schema_version"] == "0.5.0"
                        || report["schema_version"] == "0.6.0"
                        || report["schema_version"] == "0.7.0"
                        || report["schema_version"] == "0.8.0"
                        || report["schema_version"] == "0.9.0"
                        || (python_scoped_report
                            && crate::python_confirmation_recheck::valid_binding(root, &report)))
                    && event["observation"] == classify(brief, &report)
            };
            if brief["checker_id"] == "python.ruff.doctor" && !report_shape_valid {
                return Err("doctor_verification_shape_invalid");
            }
            if event["report_sha256"] != format!("{:x}", Sha256::digest(&report_bytes))
                || report["run_id"] != run_id
                || report["workspace_id"] != fact["workspace_id"]
                || report["delivery_decision"] != "not_evaluated"
                || !report_shape_valid
            {
                if brief["checker_id"] == "rust.cargo_audit" {
                    return Err(if report_shape_valid {
                        "rust_cve_verification_report_binding_invalid"
                    } else {
                        "rust_cve_verification_shape_invalid"
                    });
                }
                if brief["checker_id"] == "rust.cargo_rustdoc" {
                    return Err("rustdoc_verification_report_binding_invalid");
                }
                if matches!(
                    brief["checker_id"].as_str(),
                    Some("node.eslint" | "node.eslint.preparation")
                ) {
                    return Err(if !report_shape_valid {
                        "eslint_verification_shape_invalid"
                    } else {
                        "eslint_verification_report_binding_invalid"
                    });
                }
                return Err("verification_event_invalid");
            }
            let outcome = event["observation"]
                .as_str()
                .ok_or("verification_event_invalid")?
                .to_owned();
            let source_sha256 = if python_scoped_report && report["task_input_stable"] == true {
                report["task_binding"]["source_sha256"]
                    .as_str()
                    .filter(|sha| valid_sha256(sha))
                    .map(str::to_owned)
            } else if preparation_report && outcome == "environment_restored_unverified_policy" {
                report["scan"]["inputs"]["source"]["sha256"]
                    .as_str()
                    .filter(|s| valid_sha256(s))
                    .map(str::to_owned)
            } else if (matches!(
                outcome.as_str(),
                "still_present"
                    | "candidate_absent_unverified_policy"
                    | "suppression_requires_review"
            ) || ((go_report
                || checkstyle_report
                || brief["checker_id"] == "shell.shellcheck")
                && outcome == "rule_coverage_requires_review"))
                && brief["kind"] == "finding"
            {
                let path = brief["scope"]
                    .as_str()
                    .ok_or("verification_event_invalid")?;
                Some(
                    if checkstyle_report || brief["checker_id"] == "node.eslint" {
                        report["scan"]["inputs"]["source"]["sha256"]
                            .as_str()
                            .filter(|s| valid_sha256(s))
                            .ok_or("verification_event_invalid")?
                            .to_owned()
                    } else if matches!(
                        brief["checker_id"].as_str(),
                        Some("rust.cargo_rustdoc" | "rust.cargo_check")
                    ) {
                        report["input_identities"]
                            .as_array()
                            .and_then(|rows| rows.iter().find(|r| r["path"] == path))
                            .and_then(|r| r["sha256"].as_str())
                            .filter(|s| valid_sha256(s))
                            .ok_or("rust_verification_source_identity_invalid")?
                            .to_owned()
                    } else if brief["checker_id"] == "shell.shellcheck" {
                        report["source_sha256"]
                            .as_str()
                            .filter(|s| valid_sha256(s))
                            .ok_or("shell_verification_source_invalid")?
                            .to_owned()
                    } else if go_report {
                        report["task_target"]["source_sha256"]
                            .as_str()
                            .filter(|sha| valid_sha256(sha))
                            .ok_or("verification_event_invalid")?
                            .to_owned()
                    } else if rust_report
                        && matches!(
                            outcome.as_str(),
                            "candidate_absent_unverified_policy" | "suppression_requires_review"
                        )
                    {
                        report["suppression_probe"]["source_sha256"]
                            .as_str()
                            .filter(|sha| valid_sha256(sha))
                            .ok_or("verification_event_invalid")?
                            .to_owned()
                    } else if rust_report || java_report {
                        report["findings"]
                            .as_array()
                            .and_then(|items| items.iter().find(|item| item["finding_id"] == id))
                            .and_then(|item| item["source_sha256"].as_str())
                            .filter(|sha| valid_sha256(sha))
                            .ok_or("verification_event_invalid")?
                            .to_owned()
                    } else {
                        report["files"]
                            .as_array()
                            .and_then(|files| files.iter().find(|file| file["path"] == path))
                            .and_then(|file| file["source_sha256"].as_str())
                            .filter(|sha| valid_sha256(sha))
                            .ok_or("verification_event_invalid")?
                            .to_owned()
                    },
                )
            } else {
                None
            };
            latest_verify = Some(VerificationObservation {
                outcome,
                run_id: run_id.to_owned(),
                report_sha256: event["report_sha256"]
                    .as_str()
                    .ok_or("verification_event_invalid")?
                    .to_owned(),
                source_sha256,
                go_module_identities: go_report.then(|| report["module_identities"].clone()),
                go_task_scope: go_report.then(|| report["task_scope"].clone()),
                go_source_snapshot_sha256: if go_report {
                    report["source_snapshot_sha256"].as_str().map(str::to_owned)
                } else {
                    None
                },
                checkstyle_tool_inputs: if (checkstyle_report || preparation_report)
                    && report["scan"].is_object()
                {
                    Some(report["scan"]["inputs"].clone())
                } else {
                    None
                },
                checkstyle_reason: if checkstyle_report || preparation_report {
                    report["reason"].as_str().map(str::to_owned)
                } else {
                    None
                },
                checkstyle_source_path: if preparation_report {
                    report["target"]["path"].as_str().map(str::to_owned)
                } else {
                    None
                },
                checkstyle_config_path: if checkstyle_report || preparation_report {
                    report["scan"]["inputs"]["config"]["path"]
                        .as_str()
                        .map(str::to_owned)
                } else {
                    None
                },
                config_sha256: if checkstyle_report || preparation_report {
                    report["scan"]["inputs"]["config"]["sha256"]
                        .as_str()
                        .map(str::to_owned)
                } else if cve_report {
                    Some(
                        report["task_config_sha256"]
                            .as_str()
                            .filter(|sha| valid_sha256(sha))
                            .ok_or("verification_event_invalid")?
                            .to_owned(),
                    )
                } else {
                    None
                },
                ruff_configuration: if brief["checker_id"] == "python.ruff" {
                    report["files"]
                        .as_array()
                        .and_then(|files| files.iter().find(|file| file["path"] == brief["scope"]))
                        .and_then(|file| {
                            Some((
                                file["configuration_ref"].as_str()?.to_owned(),
                                file["config_sha256"].as_str()?.to_owned(),
                            ))
                        })
                } else {
                    None
                },
            });
        } else if sequence > latest_run {
            latest_verify = None;
        }
        latest_run = sequence;
    }
    Ok(latest_verify)
}

fn run_sequence(run_id: &str) -> Option<u128> {
    if let Some(value) = run_id.strip_prefix("shellcheck-") {
        return value.split('-').next()?.parse().ok();
    }
    if run_id.starts_with("rust-cve-") || run_id.starts_with("python-cve-") {
        return run_id.rsplit('-').nth(1)?.parse().ok();
    }
    if run_id.starts_with("rustdoc-") || run_id.starts_with("cargo-build-") {
        return run_id.rsplit('-').next()?.parse().ok();
    }
    if run_id.starts_with("eslint-") || run_id.starts_with("npm-") {
        return run_id.rsplit('-').next()?.parse().ok();
    }
    if run_id.starts_with("checkstyle-") || run_id.starts_with("syntax-confirm-") {
        return run_id.rsplit('-').next()?.parse().ok();
    }
    if let Some(value) = run_id
        .strip_prefix("rust-lint-")
        .or_else(|| run_id.strip_prefix("go-lint-"))
        .or_else(|| run_id.strip_prefix("java-p3c-"))
        .or_else(|| run_id.strip_prefix("java-cve-"))
        .or_else(|| run_id.strip_prefix("doctor-"))
    {
        let (prefix, _) = value.rsplit_once('-')?;
        prefix.rsplit_once('-')?.1.parse().ok()
    } else {
        run_id
            .strip_prefix("lint-")?
            .rsplit_once('-')?
            .1
            .parse()
            .ok()
    }
}

fn view(disposition: &str, reason: &str, brief: Value, actions: Value) -> Value {
    json!({
        "schema_version":if brief["schema_version"] == "0.20.0" {json!("0.20.0")}else if brief["checker_id"] == "shell.shellcheck" {json!("0.17.0")} else if brief["checker_id"] == "go.vet" {json!("0.13.0")} else if brief["checker_id"] == "syntax.native_confirmation" {brief["schema_version"].clone()} else {json!("0.1.0")}, "report_type":"repair_brief_preview",
        "operation":"next", "command_status":"complete", "exit_code":0,
        "disposition":disposition, "reason":reason,
        "repair_brief":brief, "next_actions":actions,
        "authority":"local_unverified", "delivery_decision":"not_evaluated"
    })
}

struct ReportQueue {
    untried: bool,
    failed: Vec<Value>,
}

fn pending_reports(
    reports: &Path,
    consumed: &Path,
    workspace_id: &str,
) -> Result<ReportQueue, &'static str> {
    let mut queue = ReportQueue {
        untried: false,
        failed: Vec::new(),
    };
    for entry in fs::read_dir(reports).map_err(|_| "reports_unreadable")? {
        let entry = entry.map_err(|_| "reports_unreadable")?;
        let path = entry.path();
        if path
            .extension()
            .is_some_and(|extension| extension == "json")
        {
            let run_id = path
                .file_stem()
                .and_then(|name| name.to_str())
                .ok_or("report_name_invalid")?;
            if !safe_run_id(run_id) {
                return Err("report_name_invalid");
            }
            let marker = consumed.join(format!("{run_id}.json"));
            if !marker.exists() {
                let bytes = read_bounded(&path, 16 * 1024 * 1024)?;
                let digest = format!("{:x}", Sha256::digest(bytes));
                let failure = consumed
                    .parent()
                    .ok_or("state_missing")?
                    .join("import-failures")
                    .join(format!("{run_id}-{digest}.json"));
                if failure.exists() {
                    let receipt: Value = serde_json::from_slice(&read_bounded(&failure, 4096)?)
                        .map_err(|_| "import_failure_receipt_invalid")?;
                    if receipt["schema_version"] != "0.1.0"
                        || receipt["record_type"] != "local_import_failure"
                        || receipt["workspace_id"] != workspace_id
                        || receipt["run_id"] != run_id
                        || receipt["report_sha256"] != digest
                        || receipt["authority"] != "local_unverified"
                        || receipt["delivery_decision"] != "not_evaluated"
                        || !receipt["reason"].as_str().is_some_and(safe_reason)
                    {
                        return Err("import_failure_receipt_invalid");
                    }
                    queue.failed.push(json!({
                        "run_id":run_id,
                        "reason":receipt["reason"],
                        "report_ref":format!(".codeguard/reports/{run_id}.json")
                    }));
                } else {
                    queue.untried = true;
                }
                continue;
            }
            let marker: Value = serde_json::from_slice(&read_bounded(&marker, 4096)?)
                .map_err(|_| "consumed_marker_invalid")?;
            let bytes = read_bounded(&path, 16 * 1024 * 1024)?;
            if marker["schema_version"] != "0.1.0"
                || marker["workspace_id"] != workspace_id
                || marker["run_id"] != run_id
                || marker["report_sha256"] != format!("{:x}", Sha256::digest(bytes))
            {
                return Err("consumed_marker_invalid");
            }
        }
    }
    queue
        .failed
        .sort_by(|left, right| left["run_id"].as_str().cmp(&right["run_id"].as_str()));
    Ok(queue)
}

fn read_bounded(path: &Path, limit: u64) -> Result<Vec<u8>, &'static str> {
    let metadata = fs::symlink_metadata(path).map_err(|_| "record_unreadable")?;
    if !metadata.file_type().is_file() || metadata.len() > limit {
        return Err("record_not_bounded_file");
    }
    fs::read(path).map_err(|_| "record_unreadable")
}

fn real_directory(path: &Path) -> bool {
    fs::symlink_metadata(path).is_ok_and(|metadata| metadata.file_type().is_dir())
}

/// 校验本地任务身份的封闭格式；格式有效不代表证据或权限已核验。
pub(crate) fn safe_id(value: &str) -> bool {
    let suffix = value
        .strip_prefix("CG-B-")
        .or_else(|| value.strip_prefix("CG-"));
    suffix.is_some_and(|part| {
        part.len() == 32
            && part
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
    })
}

fn safe_run_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 120
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

fn safe_reason(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 80
        && value
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_')
}

fn safe_rule(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 80
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-' | b':'))
}

fn safe_relative_path(value: &str) -> bool {
    !value.is_empty()
        && !value.starts_with('/')
        && !value.contains(['\\', ':'])
        && value
            .split('/')
            .all(|part| !matches!(part, "" | "." | "..") && !part.chars().any(char::is_control))
}

fn print_error(json_format: bool, reason: &str) -> ExitCode {
    if json_format {
        println!(
            "{}",
            json!({
                "schema_version":"0.1.0", "report_type":"repair_brief_preview",
                "operation":"next", "command_status":"incomplete", "exit_code":3,
                "disposition":"verification_required", "reason":reason,
                "repair_brief":null, "next_actions":[],
                "authority":"local_unverified", "delivery_decision":"not_evaluated"
            })
        );
    } else {
        eprintln!("无法核对本地修复视图：{reason}");
    }
    ExitCode::from(3)
}

fn parse_args(args: &[String]) -> Result<Arguments, String> {
    let mut root = None;
    let mut json = false;
    let mut index = 0;
    while index < args.len() {
        let arg = &args[index];
        if arg == "--format" {
            index += 1;
            json = parse_format(args.get(index).ok_or("--format 缺少值")?)?;
        } else if let Some(value) = arg.strip_prefix("--format=") {
            json = parse_format(value)?;
        } else if arg.starts_with('-') || root.is_some() {
            return Err(format!("不支持的参数：{arg}"));
        } else {
            root = Some(PathBuf::from(arg));
        }
        index += 1;
    }
    Ok(Arguments {
        root: root.unwrap_or_else(|| PathBuf::from(".")),
        json,
    })
}

fn parse_format(value: &str) -> Result<bool, String> {
    match value {
        "json" => Ok(true),
        "human" => Ok(false),
        _ => Err(format!("不支持的格式：{value}")),
    }
}

#[cfg(test)]
mod python_action_tests {
    use super::canonical_action_id;
    use serde_json::json;
    #[test]
    fn native_syntax_evidence_controls_action_even_when_budget_requires_decision() {
        let mut brief = json!({"kind":"blocker","checker_id":"python.ruff","reason_code":"python_syntax_confirmation_needed","verification_observation":"still_present","disposition":"actionable"});
        assert_eq!(canonical_action_id(&brief), Ok("repair-source"));
        brief["disposition"] = json!("needs_decision");
        assert_eq!(canonical_action_id(&brief), Ok("repair-source"));
        brief["verification_invalidated_reason"] = json!("source_input_changed_or_unavailable");
        assert_eq!(
            canonical_action_id(&brief),
            Ok("restore-checker-environment")
        );
        brief
            .as_object_mut()
            .unwrap()
            .remove("verification_invalidated_reason");
        for outcome in [
            "still_blocked",
            "candidate_absent_unverified_policy",
            "incomplete",
        ] {
            brief["verification_observation"] = json!(outcome);
            assert_eq!(
                canonical_action_id(&brief),
                Ok("restore-checker-environment")
            );
        }
        brief["verification_observation"] = json!("still_present");
        brief["reason_code"] = json!("ruff_tool_not_found");
        assert_eq!(
            canonical_action_id(&brief),
            Ok("restore-checker-environment")
        );
    }
}
