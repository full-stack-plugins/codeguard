//! 候选确认任务的原生复检与历史观察；不以局部零诊断关闭正式任务。
use codeguard_runtime::read_bounded_regular_file;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    path::{Component, Path},
    time::{Instant, SystemTime, UNIX_EPOCH},
};

/// 在原工作区和已有任务范围内运行适用语法工具；参数含显式 Zig/OTP 和共同截止时间。
/// 返回绑定当前字节的观察，缺工具/adapter 同样保留报告，便于记录失败尝试。
pub(crate) fn run(
    root: &Path,
    brief: &Value,
    zig: Option<&Path>,
    erl: Option<&Path>,
    deadline: Instant,
) -> Result<Value, &'static str> {
    let original = original(root, brief)?;
    let path = original["scope"].as_str().ok_or("syntax_scope_invalid")?;
    let language = original["language"]
        .as_str()
        .ok_or("syntax_language_invalid")?;
    if zig.is_some() && language != "zig" {
        return Err("zig_tool_does_not_match_confirmation_language");
    }
    if erl.is_some() && language != "erlang" {
        return Err("erl_tool_does_not_match_confirmation_language");
    }
    let source = source_bytes(root, path);
    let native = if let Some(bytes) = source.as_ref() {
        if language == "zig" {
            zig.and_then(|tool| crate::zig_syntax_probe::observe(tool, bytes, root, deadline))
                .unwrap_or_else(|| {
                    unavailable(if zig.is_some() {
                        "zig_tool_unavailable_or_untrusted"
                    } else {
                        "explicit_zig_tool_not_provided"
                    })
                })
        } else if language == "erlang" {
            erl.map(|tool| crate::erlang_syntax_probe::observe(tool, bytes, deadline))
                .unwrap_or_else(|| unavailable("explicit_erl_tool_not_provided"))
        } else {
            unavailable("native_syntax_confirmation_adapter_unavailable")
        }
    } else {
        unavailable("native_syntax_source_unavailable")
    };
    let mut native = native;
    if language == "erlang" && native["status"] == "not_run" {
        native["diagnostics_truncated"] = json!(false);
        native["preprocessing_unresolved"] = json!(false);
    }
    let tool_path = zig.or(erl).and_then(|p| p.canonicalize().ok());
    let target_sha = source.as_ref().map(|b| digest(b));
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| "clock_unavailable")?
        .as_nanos();
    let mut report = json!({"schema_version":if language == "erlang" {"0.2.0"} else {"0.1.0"},"report_type":"syntax_task_recheck","operation":"task_verify",
        "workspace_binding":"bound","workspace_id":original["workspace_id"],"run_id":format!("syntax-native-{}-{nanos}",std::process::id()),
        "checker_id":"syntax.native_confirmation","task_id":brief["task_id"],"authority":"local_unverified","coverage_proven":false,"delivery_decision":"not_evaluated",
        "target":{"path":path,"language":language,"source_sha256":target_sha},"original_report":{"run_id":original["run_id"],"sha256":brief["evidence_ref"]["first_report_sha256"],"source_sha256":original["observations"][0]["source_sha256"],"grammar_sha256":original["observations"][0]["grammar_sha256"]},
        "tool_path":tool_path,"native":native,"input_stable":false});
    report["input_stable"] = json!(source.is_some() && inputs_current(root, &report));
    Ok(report)
}

fn unavailable(reason: &str) -> Value {
    json!({"status":"not_run","reason":reason,"version":null,"tool_sha256":null,"diagnostics":[]})
}
fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn source_bytes(root: &Path, path: &str) -> Option<Vec<u8>> {
    if path.is_empty()
        || path.contains('\\')
        || path.chars().any(char::is_control)
        || !Path::new(path)
            .components()
            .all(|c| matches!(c, Component::Normal(_)))
    {
        return None;
    }
    let p = root.join(path);
    if p.canonicalize().ok().as_deref() != Some(p.as_path()) {
        return None;
    }
    read_bounded_regular_file(&p, 1024 * 1024).ok()
}
pub(crate) fn original(root: &Path, brief: &Value) -> Result<Value, &'static str> {
    let run = brief["evidence_ref"]["first_run_id"]
        .as_str()
        .ok_or("syntax_original_report_missing")?;
    if !run.starts_with("syntax-confirm-")
        || !run.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-')
    {
        return Err("syntax_original_report_invalid");
    }
    let bytes = read_bounded_regular_file(
        &root.join(".codeguard/reports").join(format!("{run}.json")),
        1024 * 1024,
    )
    .map_err(|_| "syntax_original_report_unavailable")?;
    if brief["evidence_ref"]["first_report_sha256"] != digest(&bytes) {
        return Err("syntax_original_report_changed");
    }
    let report = codeguard_adapters::parse_unique_json(&bytes)
        .map_err(|_| "syntax_original_report_invalid")?;
    let workspace = crate::workspace_refresh::read_workspace_baseline(root)
        .ok()
        .flatten()
        .and_then(|b| b.workspace_id().map(str::to_owned))
        .ok_or("workspace_invalid")?;
    if report["schema_version"] != "0.1.0"
        || report["report_type"] != "syntax_confirmation_observation"
        || report["workspace_id"] != workspace
        || report["run_id"] != run
        || report["blocker_id"] != brief["task_id"]
        || report["checker_id"] != "syntax.native_confirmation"
        || report["scope"] != brief["scope"]
        || !report["language"].as_str().is_some_and(|lang| {
            codeguard_adapters::bundled_grammar_candidates()
                .ok()
                .is_some_and(|m| {
                    m.assets.iter().any(|a| {
                        a.language == lang
                            && report["observations"][0]["grammar_sha256"] == a.sha256
                    })
                })
        })
    {
        return Err("syntax_original_report_invalid");
    }
    let marker = read_bounded_regular_file(
        &root
            .join(".codeguard/state/consumed")
            .join(format!("{run}.json")),
        4096,
    )
    .ok()
    .and_then(|b| codeguard_adapters::parse_unique_json(&b).ok())
    .ok_or("syntax_original_report_not_synced")?;
    if marker["workspace_id"] != workspace
        || marker["run_id"] != run
        || marker["report_sha256"] != digest(&bytes)
    {
        return Err("syntax_original_report_not_synced");
    }
    Ok(report)
}

/// 保存或投影前重新核对当前源文件和已观察工具身份；不验证策略或完整覆盖。
pub(crate) fn inputs_current(root: &Path, report: &Value) -> bool {
    report["target"]["path"]
        .as_str()
        .and_then(|p| source_bytes(root, p))
        .is_some_and(|b| report["target"]["source_sha256"] == digest(&b))
        && (report["native"]["tool_sha256"].is_null() || tool_current(report))
}

fn tool_current(report: &Value) -> bool {
    report["native"]["tool_sha256"]
        .as_str()
        .is_some_and(valid_sha)
        && report["tool_path"].as_str().is_some_and(|p| {
            let path = Path::new(p);
            path.is_absolute()
                && path.canonicalize().ok().as_deref() == Some(path)
                && read_bounded_regular_file(path, 64 * 1024 * 1024)
                    .ok()
                    .is_some_and(|b| report["native"]["tool_sha256"] == digest(&b))
        })
}

/// 只分类本轮局部语法结果；原生零诊断仍等待正式策略与覆盖核验。
pub(crate) fn classify(report: &Value) -> &'static str {
    if report["input_stable"] != true {
        "incomplete"
    } else {
        match report["native"]["status"].as_str() {
            Some("diagnostics_observed") => "still_blocked",
            Some("completed") => "candidate_absent_unverified_policy",
            _ => "incomplete",
        }
    }
}

/// 校验新报告的固定键、任务归属与当前输入；仅允许历史观察导入。
pub(crate) fn valid_shape(root: &Path, report: &Value) -> bool {
    valid_history_shape(root, report)
        && (report["input_stable"] == false || inputs_current(root, report))
}

fn valid_history_shape(root: &Path, report: &Value) -> bool {
    let keys = [
        "schema_version",
        "report_type",
        "operation",
        "workspace_binding",
        "workspace_id",
        "run_id",
        "checker_id",
        "task_id",
        "authority",
        "coverage_proven",
        "delivery_decision",
        "target",
        "original_report",
        "tool_path",
        "native",
        "input_stable",
    ];
    if !report
        .as_object()
        .is_some_and(|o| o.len() == keys.len() && keys.iter().all(|k| o.contains_key(*k)))
        || !matches!(report["schema_version"].as_str(), Some("0.1.0" | "0.2.0"))
        || report["report_type"] != "syntax_task_recheck"
        || report["operation"] != "task_verify"
        || report["workspace_binding"] != "bound"
        || report["checker_id"] != "syntax.native_confirmation"
        || report["authority"] != "local_unverified"
        || report["coverage_proven"] != false
        || report["delivery_decision"] != "not_evaluated"
        || !report["input_stable"].is_boolean()
    {
        return false;
    }
    let Some(id) = report["task_id"].as_str().filter(|id| {
        id.strip_prefix("CG-B-").is_some_and(|s| {
            s.len() == 32
                && s.bytes()
                    .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
        })
    }) else {
        return false;
    };
    let Some(run) = report["run_id"]
        .as_str()
        .and_then(|r| r.strip_prefix("syntax-native-"))
    else {
        return false;
    };
    if !run
        .split_once('-')
        .is_some_and(|(p, n)| p.parse::<u32>().is_ok() && n.parse::<u128>().is_ok())
    {
        return false;
    }
    let fact = read_bounded_regular_file(
        &root.join(format!(".codeguard/findings/{id}/finding.json")),
        128 * 1024,
    )
    .ok()
    .and_then(|b| codeguard_adapters::parse_unique_json(&b).ok());
    let Some(fact) = fact else {
        return false;
    };
    if fact["checker_id"] != "syntax.native_confirmation"
        || fact["kind"] != "blocker"
        || !native_shape(root, report)
    {
        return false;
    }
    let brief = json!({"task_id":id,"scope":fact["scope"],"evidence_ref":{"first_run_id":fact["first_run_id"],"first_report_sha256":fact["first_report_sha256"]}});
    let Ok(old) = original(root, &brief) else {
        return false;
    };
    report["workspace_id"] == old["workspace_id"]
        && report["target"]["path"] == old["scope"]
        && report["target"]["language"] == old["language"]
        && report["original_report"]
            == json!({"run_id":old["run_id"],"sha256":fact["first_report_sha256"],"source_sha256":old["observations"][0]["source_sha256"],"grammar_sha256":old["observations"][0]["grammar_sha256"]})
        && matches!(
            report["native"]["status"].as_str(),
            Some("not_run" | "incomplete" | "completed" | "diagnostics_observed")
        )
}

fn native_shape(root: &Path, report: &Value) -> bool {
    if report["schema_version"] == "0.2.0" {
        let current = report["target"]["path"]
            .as_str()
            .and_then(|p| source_bytes(root, p))
            .filter(|b| report["target"]["source_sha256"] == digest(b));
        return report["target"]["language"] == "erlang"
            && report["target"].as_object().is_some_and(|o| {
                o.len() == 3
                    && ["path", "language", "source_sha256"]
                        .iter()
                        .all(|k| o.contains_key(*k))
            })
            && (report["target"]["source_sha256"].is_null()
                || report["target"]["source_sha256"]
                    .as_str()
                    .is_some_and(valid_sha))
            && (report["tool_path"].is_null()
                || report["tool_path"]
                    .as_str()
                    .is_some_and(|s| Path::new(s).is_absolute()))
            && (!matches!(
                report["native"]["status"].as_str(),
                Some("completed" | "diagnostics_observed")
            ) || (report["target"]["source_sha256"]
                .as_str()
                .is_some_and(valid_sha)
                && !report["tool_path"].is_null()))
            && crate::erlang_syntax_probe::valid_native_observation(
                &report["native"],
                current.as_deref(),
            );
    }
    let native = &report["native"];
    let Some(status) = native["status"].as_str() else {
        return false;
    };
    let base = ["status", "reason", "version", "tool_sha256", "diagnostics"];
    if !native.as_object().is_some_and(|o| {
        (o.len() == base.len() || o.len() == base.len() + 1)
            && base.iter().all(|k| o.contains_key(*k))
            && o.keys()
                .all(|k| base.contains(&k.as_str()) || k == "diagnostic_count")
    }) || !native["reason"].as_str().is_some_and(|s| {
        !s.is_empty() && s.len() <= 128 && s.bytes().all(|b| b.is_ascii_lowercase() || b == b'_')
    }) {
        return false;
    }
    let Some(rows) = native["diagnostics"].as_array().filter(|r| r.len() <= 32) else {
        return false;
    };
    if let Some(n) = native["diagnostic_count"].as_u64() {
        if n != rows.len() as u64 {
            return false;
        }
    }
    if matches!(status, "completed" | "diagnostics_observed")
        && (report["target"]["language"] != "zig"
            || native["version"] != "0.16.0"
            || !native["tool_sha256"].as_str().is_some_and(valid_sha)
            || !native["diagnostic_count"].is_u64()
            || report["tool_path"].is_null()
            || !report["target"]["source_sha256"]
                .as_str()
                .is_some_and(valid_sha)
            || (status == "completed" && !rows.is_empty())
            || (status == "completed" && native["reason"] != "ast_check_no_diagnostics")
            || (status == "diagnostics_observed" && rows.is_empty())
            || (status == "diagnostics_observed" && native["reason"] != "ast_check_diagnostics"))
    {
        return false;
    }
    if !(native["tool_sha256"].is_null() || native["tool_sha256"].as_str().is_some_and(valid_sha))
        || !(native["version"].is_null() || native["version"] == "0.16.0")
        || (native.get("diagnostic_count").is_some() && !native["diagnostic_count"].is_u64())
    {
        return false;
    }
    if status == "not_run"
        && (!rows.is_empty() || !native["tool_sha256"].is_null() || !native["version"].is_null())
    {
        return false;
    }
    if !report["target"].as_object().is_some_and(|o| {
        o.len() == 3
            && ["path", "language", "source_sha256"]
                .iter()
                .all(|k| o.contains_key(*k))
    }) || !(report["target"]["source_sha256"].is_null()
        || report["target"]["source_sha256"]
            .as_str()
            .is_some_and(valid_sha))
        || !(report["tool_path"].is_null()
            || report["tool_path"]
                .as_str()
                .is_some_and(|s| Path::new(s).is_absolute()))
    {
        return false;
    }
    let current = report["target"]["path"]
        .as_str()
        .and_then(|p| source_bytes(root, p))
        .filter(|b| report["target"]["source_sha256"] == digest(b));
    rows.iter().all(|r| {
        r.as_object().is_some_and(|o| {
            o.len() == 3
                && ["line", "column", "rule_id"]
                    .iter()
                    .all(|k| o.contains_key(*k))
        }) && r["rule_id"] == "zig.ast_check.error"
            && r["line"]
                .as_u64()
                .is_some_and(|n| n > 0 && n <= u32::MAX as u64)
            && r["column"]
                .as_u64()
                .is_some_and(|n| n > 0 && n <= 1024 * 1024 + 1)
            && current.as_ref().is_none_or(|bytes| {
                let line = r["line"].as_u64().unwrap() as usize;
                let col = r["column"].as_u64().unwrap() as usize;
                bytes
                    .split(|b| *b == b'\n')
                    .nth(line - 1)
                    .is_some_and(|l| col <= l.len() + 1)
            })
    })
}
fn valid_sha(s: &str) -> bool {
    s.len() == 64
        && s.bytes()
            .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
}

/// 读取同任务已消费的最新原生观察，为 next 提供当前动作；原输入失效时优先复检。
pub(crate) fn guidance(root: &Path, brief: &Value) -> Option<Value> {
    let id = brief["task_id"].as_str()?;
    let entries = std::fs::read_dir(root.join(format!(".codeguard/findings/{id}/events"))).ok()?;
    let mut latest = None;
    for (index, entry) in entries.enumerate() {
        if index >= 1000 {
            return Some(
                json!({"disposition":"verification_required","step":"语法确认历史超过预算；先诊断记录，不沿用旧原生观察"}),
            );
        }
        let entry = entry.ok()?;
        let name = entry.file_name().into_string().ok()?;
        let Some(run) = name
            .strip_prefix("verify-")
            .and_then(|s| s.strip_suffix(".json"))
            .filter(|s| s.starts_with("syntax-native-"))
        else {
            continue;
        };
        let sequence = run.rsplit('-').next()?.parse::<u128>().ok()?;
        if latest.as_ref().is_some_and(|(n, _, _)| *n >= sequence) {
            continue;
        }
        let event = read_bounded_regular_file(&entry.path(), 4096)
            .ok()
            .and_then(|b| codeguard_adapters::parse_unique_json(&b).ok())?;
        let bytes = read_bounded_regular_file(
            &root.join(".codeguard/reports").join(format!("{run}.json")),
            1024 * 1024,
        )
        .ok()?;
        let report = codeguard_adapters::parse_unique_json(&bytes).ok()?;
        let marker = read_bounded_regular_file(
            &root
                .join(".codeguard/state/consumed")
                .join(format!("{run}.json")),
            4096,
        )
        .ok()
        .and_then(|b| codeguard_adapters::parse_unique_json(&b).ok())?;
        if !valid_history_shape(root, &report)
            || event["schema_version"] != "0.3.0"
            || event["event"] != "verification_observed"
            || event["workspace_id"] != report["workspace_id"]
            || event["authority"] != "local_unverified"
            || event["state_after"] != "open"
            || event["kind"] != "blocker"
            || event["observation"] != classify(&report)
            || event["task_id"] != id
            || event["run_id"] != run
            || event["report_sha256"] != digest(&bytes)
            || marker["report_sha256"] != digest(&bytes)
            || marker["schema_version"] != "0.1.0"
            || marker["workspace_id"] != report["workspace_id"]
            || marker["run_id"] != run
            || report["task_id"] != id
        {
            return None;
        }
        latest = Some((sequence, report, digest(&bytes)));
    }
    let (_, report, report_sha256) = latest?;
    let (disposition, step) = if !inputs_current(root, &report) {
        (
            "verification_required",
            "源码或原生工具在语法复检后已变化；先运行当前输入的原生确认，不沿用旧修复或零诊断",
        )
    } else if classify(&report) == "still_blocked" {
        (
            "actionable",
            if report["target"]["language"] == "erlang" {
                "当前源码已有原生 Erlang 语法诊断；核对报告中有界原生位置并修复，然后使用同一工具复检；不要反复安装工具或关闭检查"
            } else {
                "当前源码已有原生 Zig AST 诊断；核对报告中有界原生位置并修复，然后使用同一工具复检；不要反复安装工具或关闭检查"
            },
        )
    } else if classify(&report) == "candidate_absent_unverified_policy" {
        (
            "verification_required",
            "原生语法复检未发现诊断；核对原输入的反证或修复归因及正式策略/覆盖，保留证据，不重复源码修复或安装工具，也不自动关闭",
        )
    } else {
        (
            "needs_decision",
            if report["target"]["language"] == "erlang"
                && report["native"]["reason"] == "erlang_preprocessing_unresolved"
            {
                "Erlang 文件需要宏、条件编译或 include 上下文；当前 forms 解析不能完成确认，先恢复项目原生编译或预处理，不据此修改无关源码"
            } else if report["target"]["language"] == "erlang"
                && report["native"]["reason"] == "explicit_erl_tool_not_provided"
            {
                "未提供 Erlang 原生工具；先定位已安装的 OTP 28 erl，或按项目要求安装匹配工具，再用 --erl-tool 绝对路径复检，不修改无关源码"
            } else {
                "原生语法确认仍未完成；查看原工具诊断、语言能力或版本缺口，恢复对应前置，不修改无关源码"
            },
        )
    };
    let mut guidance = json!({"disposition":disposition,"step":step,"native_confirmation_status":if inputs_current(root,&report) {report["native"]["status"].clone()} else {json!("stale")}});
    if report["target"]["language"] == "erlang" {
        guidance["schema_version"] = json!("0.4.0");
        guidance["native_column_unit"] = json!("unicode_scalar");
        guidance["native_confirmation_reason"] = if inputs_current(root, &report) {
            report["native"]["reason"].clone()
        } else {
            json!("syntax_confirmation_inputs_changed")
        };
    }
    guidance["native_confirmation_ref"] = json!({"run_id":report["run_id"],"report_ref":format!(".codeguard/reports/{}.json", report["run_id"].as_str()?),"report_sha256":report_sha256});
    guidance["native_diagnostic_positions"] =
        if inputs_current(root, &report) && classify(&report) == "still_blocked" {
            report["native"]["diagnostics"].clone()
        } else {
            json!([])
        };
    // 源码修复后仍可复用未改变的工具；工具字节变化则不得携带旧工具身份。
    if matches!(
        report["target"]["language"].as_str(),
        Some("zig" | "erlang")
    ) && tool_current(&report)
        && (report["target"]["language"] != "erlang" || report["native"]["version"] == "OTP 28")
    {
        guidance["recheck_argv"] = json!([
            "codeguard",
            "task",
            "verify",
            id,
            ".",
            "--format",
            "json",
            if report["target"]["language"] == "erlang" {
                "--erl-tool"
            } else {
                "--zig-tool"
            },
            report["tool_path"]
        ]);
    }
    Some(guidance)
}
