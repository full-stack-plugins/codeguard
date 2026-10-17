//! 原生Go缺失时的有界候选反馈；复用固定grammar、稳定任务和共同截止时间。
use serde_json::{Value, json};
use std::{path::Path, process::ExitCode, time::Instant};

/// 补充局部原生报告的候选初检；输出对话报告但不保存为原生执行证据。
pub(crate) fn run(
    root: &Path,
    native: Value,
    selection: Value,
    deadline: Instant,
    json_output: bool,
) -> ExitCode {
    // 取消后不得继续启动 WASM 候选初检；保留取消语义并按契约返回 130。
    if native["reason"] == "request_cancelled" || codeguard_runtime::sigint_cancellation_requested()
    {
        let report = json!({"schema_version":"0.7.0","report_type":"go_lint_fallback_feedback","operation":"lint","language":"go",
            "command_status":"cancelled","exit_code":130,"authority":"local_unverified","coverage_proven":false,"delivery_decision":"not_evaluated",
            "tool_selection":selection,"native_report":native,
            "syntax_candidates":{"status":"not_run","reason":"request_cancelled","execution_phase":"after_native","authority":"candidate_unqualified","delivery_decision":"incomplete",
                "source_file_count":0,"skipped_count":0,"unrouted_count":0,"native_preferred_count":0,"observations":[],
                "next_action":"请求已取消；不缓存也不签发 clean，需要时重新执行原检查"},
            "syntax_tasks":{"status":"not_attempted"},
            "preliminary_result":"incomplete","native_tool_requirement":"required",
            "next_actions":["request_cancelled_rerun_after_review"]});
        if json_output {
            println!("{report}");
        } else {
            println!("Go 检查已取消（退出 130）；候选初检未运行，不签发任何通过结论");
        }
        return ExitCode::from(130);
    }
    let syntax = observe(root, deadline);
    let tasks = crate::syntax_confirmation::persist(root, &syntax, deadline);
    let result = preliminary_result(&syntax);
    let requirement = if result == "no_candidates_observed" {
        "recommended"
    } else {
        "required"
    };
    let mut report = json!({"schema_version":"0.7.0","report_type":"go_lint_fallback_feedback","operation":"lint","language":"go",
        "command_status":"incomplete","exit_code":3,"authority":"local_unverified","coverage_proven":false,"delivery_decision":"not_evaluated",
        "tool_selection":selection,"native_report":native,"syntax_candidates":syntax,"syntax_tasks":tasks,
        "preliminary_result":result,"native_tool_requirement":requirement,
        "next_actions":if requirement == "required" {json!(["prepare_project_native_go_then_confirm_candidates","preserve_source_until_native_confirmation"])}else {json!(["recommend_project_native_go_for_complete_lint","retain_unfinished_project_obligations"])}});
    // 候选初检期间的取消同样保留取消语义；已取得的候选观察不撤回也不升级为通过。
    crate::go_lint_command::mark_request_cancellation(&mut report);
    let exit = report["exit_code"].as_u64().unwrap_or(3) as u8;
    if json_output {
        println!("{report}");
    } else {
        println!(
            "Go 原生lint未运行；WASM完整文件候选初检：{}；覆盖与交付仍未完成",
            report["preliminary_result"]
        );
        println!(
            "原生工具准备：{}；先核对项目Go版本和已安装SDK，再用 --go-tool 绝对路径复检",
            if requirement == "required" {
                "必须准备"
            } else {
                "建议准备"
            }
        );
        for row in report["syntax_candidates"]["observations"]
            .as_array()
            .into_iter()
            .flatten()
            .take(8)
        {
            println!(
                "文件 {}；原始恢复 {}；独立结构候选 {}；状态 {}",
                row["path"],
                row["recovery_count"],
                row["structural_observation_count"].as_u64().unwrap_or(0),
                row["status"]
            );
        }
        println!(
            "修复任务同步：{}；不凭候选修改无关源码，不以零候选关闭任务",
            report["syntax_tasks"]["status"]
        );
    }
    ExitCode::from(exit)
}

#[cfg(feature = "wasm-precheck")]
fn observe(root: &Path, deadline: Instant) -> Value {
    let Ok(registry) = codeguard_adapters::legacy_registry() else {
        return unavailable("grammar_registry_unavailable");
    };
    let discovery =
        crate::discovery::discover(root, &registry, &codeguard_runtime::NativeObservation);
    crate::check_syntax_candidates::observe(
        root,
        &discovery,
        crate::check_syntax_candidates::NativeCoverage {
            node_lint: &Value::Null,
            python_lint: &Value::Null,
            go_lint: &Value::Null,
            erlang_lint: &Value::Null,
            kotlin_lint: &Value::Null,
            zig_lint: &Value::Null,
            swift_lint: &Value::Null,
            ruby_lint: &Value::Null,
            rust_targets: &crate::rust_native_syntax_coverage::RustNativeSyntaxCoverage::default(),
            go_tool: None,
        },
        Some("go"),
        2,
        deadline,
        if discovery.observation_complete {
            None
        } else {
            Some("go_source_discovery_incomplete")
        },
    )
}
#[cfg(not(feature = "wasm-precheck"))]
fn observe(_root: &Path, _deadline: Instant) -> Value {
    unavailable("wasm_feature_not_built")
}
fn unavailable(reason: &str) -> Value {
    json!({"status":"not_run","reason":reason,"execution_phase":"after_native","authority":"candidate_unqualified","delivery_decision":"incomplete",
        "source_file_count":0,"skipped_count":0,"unrouted_count":0,"native_preferred_count":0,"observations":[],
        "next_action":"准备项目适用Go原生工具；初检不可用不能认定源码通过"})
}
fn preliminary_result(syntax: &Value) -> &'static str {
    let Some(rows) = syntax["observations"].as_array() else {
        return "incomplete";
    };
    let candidate = rows.iter().any(|row| {
        row["recovery_count"].as_u64().unwrap_or(0) > 0
            || row["structural_observation_count"].as_u64().unwrap_or(0) > 0
    });
    if candidate {
        return "candidates_observed";
    }
    if syntax["status"] == "observed_partial"
        && syntax["source_file_count"]
            .as_u64()
            .is_some_and(|n| n > 0 && n == rows.len() as u64)
        && syntax["skipped_count"] == 0
        && syntax["unrouted_count"] == 0
        && rows
            .iter()
            .all(|row| row["status"] == "candidate_observed" && row["reason"].is_null())
    {
        "no_candidates_observed"
    } else {
        "incomplete"
    }
}

#[cfg(test)]
mod tests {
    use super::{preliminary_result, unavailable};
    use serde_json::json;
    #[test]
    fn empty_skipped_failed_or_unbuilt_observations_require_native_tools() {
        assert_eq!(
            preliminary_result(&unavailable("wasm_feature_not_built")),
            "incomplete"
        );
        let good = json!({"status":"observed_partial","source_file_count":1,"skipped_count":0,"unrouted_count":0,
            "observations":[{"status":"candidate_observed","reason":null,"recovery_count":0}]});
        assert_eq!(preliminary_result(&good), "no_candidates_observed");
        for (field, value) in [
            ("skipped_count", json!(1)),
            ("unrouted_count", json!(1)),
            ("source_file_count", json!(2)),
            ("status", json!("not_run")),
        ] {
            let mut bad = good.clone();
            bad[field] = value;
            assert_eq!(preliminary_result(&bad), "incomplete");
        }
        let mut bad = good.clone();
        bad["observations"][0]["reason"] = json!("syntax_recovery_incomplete");
        assert_eq!(preliminary_result(&bad), "incomplete");
    }
}
