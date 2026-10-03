//! OTP 28 单文件原生 forms 解析；不展开宏、不编译或执行项目源码。

use codeguard_runtime::{ProcessSpec, Termination, read_bounded_regular_file, run_process};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap, ffi::OsString, path::Path, sync::atomic::AtomicBool, time::Instant,
};

// 输入仅来自受控 stdin，eval 中没有源码插值；逐 form 使用 OTP 自带扫描和解析器。
// 预处理或宏仅标记覆盖未完成，不展开它们，也不调用 compiler/parse_transform。
const PARSE_FORMS: &str = r#"
io:setopts(standard_io,[{encoding,unicode}]),
IsPre=fun(Tokens)->
  lists:any(fun(T)->element(1,T)=:='?' end,Tokens) orelse
  case Tokens of
    [{'-',_},Name|_] ->
      Key=case Name of {atom,_,Atom}->Atom;_->element(1,Name) end,
      lists:member(Key,[define,undef,include,include_lib,ifdef,ifndef,'if',elif,'else',endif]);
    _ -> false
  end
end,
AddError=fun(Error,Errors)->case length(Errors)<33 of true->[Error|Errors];false->Errors end end,
ReadForms=fun Again(Location,Count,Pre,Errors)->
  case io:scan_erl_form(standard_io,"",Location) of
    {ok,Tokens,Next}->
      Current=case erl_parse:parse_form(Tokens) of
        {ok,_}->Errors;
        {error,Error}->AddError(Error,Errors)
      end,
      Again(Next,Count+1,Pre orelse IsPre(Tokens),Current);
    {error,Error,Next}->Again(Next,Count+1,Pre,AddError(Error,Errors));
    {eof,_}->{Count,Pre,lists:reverse(Errors)}
  end
end,
{Count,Pre,Errors}=ReadForms({1,1},0,false,[]),
Diagnostics=[begin {{Line,Column},_,_}=Error,
  #{line=>Line,column=>Column,rule_id=><<"erlang.syntax.error">>}
end||Error<-lists:sublist(Errors,32)],
io:put_chars(json:encode(#{schema_version=><<"0.1.0">>,forms=>Count,
  preprocessing=>Pre,diagnostics_truncated=>length(Errors)>32,diagnostics=>Diagnostics})),
halt().
"#;

/// 调用显式原生工具并核对返回位置、工具前后摘要和共享预算。
/// 参数为 erl 工具、本轮 UTF-8 字节和绝对截止时间；固定 cwd 不加载项目模块。
pub(crate) fn observe(tool: &Path, source: &[u8], deadline: Instant) -> Value {
    let mut report = json!({
        "status":"incomplete", "reason":"erlang_tool_unavailable_or_untrusted",
        "version":null, "tool_sha256":null, "diagnostics":[],
        "diagnostics_truncated":false, "preprocessing_unresolved":false
    });
    if !tool.is_absolute() {
        return report;
    }
    let Ok(executable) = tool.canonicalize() else {
        return report;
    };
    let Ok(bytes) = read_bounded_regular_file(&executable, 64 * 1024 * 1024) else {
        return report;
    };
    let tool_sha = format!("{:x}", Sha256::digest(bytes));
    report["tool_sha256"] = json!(tool_sha);
    let cancelled = AtomicBool::new(false);
    let invoke = |eval: &str, stdin| {
        run_process(
            &ProcessSpec {
                executable: executable.clone(),
                args: [
                    "+S",
                    "1:1",
                    "+A",
                    "1",
                    "-noshell",
                    "-no_dot_erlang",
                    "-eval",
                    eval,
                ]
                .map(OsString::from)
                .to_vec(),
                cwd: Path::new("/").to_path_buf(),
                env: BTreeMap::new(),
                stdin,
                deadline,
                output_limit_bytes: 64 * 1024,
            },
            &cancelled,
        )
    };
    let version = invoke(
        "io:format(\"OTP ~s~n\", [erlang:system_info(otp_release)]), halt().",
        None,
    );
    if version.termination != Termination::Exited(0) {
        report["reason"] = json!(execution_reason(version.termination));
        return report;
    }
    if !version.stderr.is_empty()
        || std::str::from_utf8(&version.stdout).ok().map(str::trim) != Some("OTP 28")
    {
        report["reason"] = json!("erlang_version_unverified_or_unsupported");
        return report;
    }
    report["version"] = json!("OTP 28");
    let outcome = invoke(PARSE_FORMS, Some(source.to_vec()));
    let current_sha = read_bounded_regular_file(&executable, 64 * 1024 * 1024)
        .ok()
        .map(|bytes| format!("{:x}", Sha256::digest(bytes)));
    if current_sha.as_deref() != Some(tool_sha.as_str()) {
        report["reason"] = json!("erlang_tool_changed_during_check");
        return report;
    }
    if outcome.termination != Termination::Exited(0) {
        report["reason"] = json!(execution_reason(outcome.termination));
        return report;
    }
    if !outcome.stderr.is_empty() {
        report["reason"] = json!("erlang_native_stderr_unresolved");
        return report;
    }
    let Ok(parsed) = codeguard_adapters::parse_unique_json(&outcome.stdout) else {
        report["reason"] = json!("erlang_syntax_report_invalid");
        return report;
    };
    if !validate(&parsed, source) {
        report["reason"] = json!("erlang_syntax_report_invalid");
        return report;
    }
    report["diagnostics_truncated"] = parsed["diagnostics_truncated"].clone();
    report["preprocessing_unresolved"] = parsed["preprocessing"].clone();
    // 宏或条件编译可改变 token 流，此阶段的 error marker 不能确认为源码违规。
    if parsed["preprocessing"] == true {
        report["reason"] = json!("erlang_preprocessing_unresolved");
        return report;
    }
    report["diagnostics"] = parsed["diagnostics"].clone();
    if parsed["diagnostics_truncated"] == true {
        report["reason"] = json!("erlang_diagnostic_budget_exhausted");
    } else if !parsed["diagnostics"]
        .as_array()
        .expect("已验证数组")
        .is_empty()
    {
        report["status"] = json!("diagnostics_observed");
        report["reason"] = json!("erlang_native_syntax_diagnostics");
    } else if parsed["forms"] == 0 {
        report["reason"] = json!("erlang_no_forms_observed");
    } else {
        report["status"] = json!("completed");
        report["reason"] = json!("erlang_native_forms_no_diagnostics");
    }
    report
}

fn validate(parsed: &Value, source: &[u8]) -> bool {
    let Some(fields) = parsed.as_object() else {
        return false;
    };
    if fields.len() != 5
        || parsed["schema_version"] != "0.1.0"
        || parsed["forms"].as_u64().is_none_or(|count| count > 200_000)
        || !parsed["preprocessing"].is_boolean()
        || !parsed["diagnostics_truncated"].is_boolean()
    {
        return false;
    }
    let Some(diagnostics) = parsed["diagnostics"].as_array() else {
        return false;
    };
    if diagnostics.len() > 32 || diagnostics.len() as u64 > parsed["forms"].as_u64().unwrap_or(0) {
        return false;
    }
    let Ok(text) = std::str::from_utf8(source) else {
        return false;
    };
    let lines: Vec<_> = text.split('\n').collect();
    diagnostics.iter().all(|diagnostic| {
        let (Some(line), Some(column)) =
            (diagnostic["line"].as_u64(), diagnostic["column"].as_u64())
        else {
            return false;
        };
        diagnostic
            .as_object()
            .is_some_and(|fields| fields.len() == 3)
            && diagnostic["rule_id"] == "erlang.syntax.error"
            && line > 0
            && column > 0
            && usize::try_from(line - 1)
                .ok()
                .and_then(|index| lines.get(index))
                .is_some_and(|line| column <= line.chars().count() as u64 + 1)
    })
}

/// 校验已保存原生观察的严格身份与可用范围；当前字节存在时核对 Unicode 列坐标。
/// 历史源码已变化时只保留有界记录，不能把它当作当前修复或关闭依据。
pub(crate) fn valid_native_observation(native: &Value, current: Option<&[u8]>) -> bool {
    let keys = [
        "status",
        "reason",
        "version",
        "tool_sha256",
        "diagnostics",
        "diagnostics_truncated",
        "preprocessing_unresolved",
    ];
    if !native
        .as_object()
        .is_some_and(|o| o.len() == keys.len() && keys.iter().all(|k| o.contains_key(*k)))
        || !matches!(
            native["status"].as_str(),
            Some("not_run" | "incomplete" | "completed" | "diagnostics_observed")
        )
        || !native["reason"].as_str().is_some_and(|s| {
            !s.is_empty()
                && s.len() <= 128
                && s.bytes().all(|b| b.is_ascii_lowercase() || b == b'_')
        })
        || !(native["version"].is_null() || native["version"] == "OTP 28")
        || !(native["tool_sha256"].is_null()
            || native["tool_sha256"].as_str().is_some_and(|s| {
                s.len() == 64
                    && s.bytes()
                        .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
            }))
        || !native["diagnostics_truncated"].is_boolean()
        || !native["preprocessing_unresolved"].is_boolean()
    {
        return false;
    }
    let Some(rows) = native["diagnostics"].as_array().filter(|r| r.len() <= 32) else {
        return false;
    };
    match native["status"].as_str() {
        Some("completed" | "diagnostics_observed") => {
            if native["version"] != "OTP 28"
                || native["tool_sha256"].is_null()
                || native["diagnostics_truncated"] != false
                || native["preprocessing_unresolved"] != false
                || (native["status"] == "completed"
                    && (!rows.is_empty()
                        || native["reason"] != "erlang_native_forms_no_diagnostics"))
                || (native["status"] == "diagnostics_observed"
                    && (rows.is_empty() || native["reason"] != "erlang_native_syntax_diagnostics"))
            {
                return false;
            }
        }
        Some("not_run")
            if !rows.is_empty()
                || !native["tool_sha256"].is_null()
                || !native["version"].is_null()
                || native["diagnostics_truncated"] != false
                || native["preprocessing_unresolved"] != false =>
        {
            return false;
        }
        _ => {}
    }
    if native["preprocessing_unresolved"] == true
        && (!rows.is_empty()
            || native["status"] != "incomplete"
            || native["reason"] != "erlang_preprocessing_unresolved")
    {
        return false;
    }
    if native["diagnostics_truncated"] == true && native["status"] != "incomplete" {
        return false;
    }
    rows.iter().all(|r| {
        r.as_object().is_some_and(|o| {
            o.len() == 3
                && ["line", "column", "rule_id"]
                    .iter()
                    .all(|k| o.contains_key(*k))
        }) && r["rule_id"] == "erlang.syntax.error"
            && r["line"]
                .as_u64()
                .is_some_and(|n| n > 0 && n <= u32::MAX as u64)
            && r["column"]
                .as_u64()
                .is_some_and(|n| n > 0 && n <= 1024 * 1024 + 1)
            && current.is_none_or(|bytes| {
                std::str::from_utf8(bytes)
                    .ok()
                    .and_then(|s| s.split('\n').nth(r["line"].as_u64().unwrap() as usize - 1))
                    .is_some_and(|line| {
                        r["column"].as_u64().unwrap() <= line.chars().count() as u64 + 1
                    })
            })
    })
}

fn execution_reason(termination: Termination) -> &'static str {
    match termination {
        Termination::TimedOut | Termination::DeadlineBeforeStart => "request_deadline_exceeded",
        Termination::Cancelled => "request_cancelled",
        Termination::OutputLimit => "erlang_native_output_budget_exhausted",
        Termination::SpawnFailure => "erlang_native_process_unavailable",
        Termination::CleanupFailure => "erlang_native_cleanup_incomplete",
        _ => "erlang_native_execution_incomplete",
    }
}
