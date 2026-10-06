//! 编辑事件只检查指定文件；原生检查优先，WASM 候选补充，始终保留范围缺口。

use crate::check_eslint_scan::{CheckEslintScan, is_source};
use crate::python_lint_command::scan_selected_report_with_deadline;
use serde_json::{Value, json};
use std::{collections::BTreeSet, path::Path, sync::atomic::AtomicBool, time::Instant};

/// 在共同截止时间内观察受限编辑范围；参数为根、相对路径和可选原生工具。
/// 返回局部结果及逐文件后续指引，不执行项目构建，也不签发交付结论。
pub(crate) fn observe(
    root: &Path,
    requested: &[String],
    tools: crate::hook_native_tools::HookNativeTools<'_>,
    deadline: Instant,
) -> Value {
    let mut selected = BTreeSet::new();
    let mut unavailable = Vec::new();
    for path in requested {
        if Instant::now() >= deadline {
            unavailable.push(json!({"path":path,"reason":"request_deadline_exceeded"}));
        } else if safe_file(root, path) {
            selected.insert(path.clone());
        } else {
            unavailable.push(json!({"path":path,"reason":"source_path_unavailable_or_unsafe"}));
        }
    }
    let cancelled = AtomicBool::new(false);
    let python: Vec<String> = selected
        .iter()
        .filter(|p| p.ends_with(".py"))
        .cloned()
        .collect();
    let python_lint = if python.is_empty() {
        Value::Null
    } else {
        scan_selected_report_with_deadline(root, tools.ruff, &python, deadline, &cancelled)
            .unwrap_or_else(|_| json!({"status":"incomplete","reason":"adapter_unavailable"}))
    };
    let javascript = selected.iter().filter(|p| is_source(p)).cloned().collect();
    let mut node_scan = CheckEslintScan::run(root, &javascript, tools.node, deadline, &cancelled);
    node_scan.sync(root, deadline);
    let node_lint = node_scan.feedback;
    let kotlin_paths = selected
        .iter()
        .filter(|p| p.ends_with(".kt"))
        .cloned()
        .collect::<BTreeSet<_>>();
    let mut kotlin_lint = if kotlin_paths.is_empty() {
        Value::Null
    } else {
        crate::check_kotlin_scan::observe(
            root,
            &kotlin_paths,
            tools.kotlinc.map(Path::to_path_buf),
            deadline,
            &cancelled,
        )
    };
    if kotlin_lint.is_object() {
        crate::native_syntax_confirmation::connect(root, &mut kotlin_lint, deadline);
        crate::check_kotlin_scan::refresh(root, &mut kotlin_lint, deadline);
    }
    let swift_paths = selected
        .iter()
        .filter(|p| p.ends_with(".swift"))
        .cloned()
        .collect::<BTreeSet<_>>();
    let mut swift_lint = if swift_paths.is_empty() {
        Value::Null
    } else {
        crate::check_swift_scan::observe(
            root,
            &swift_paths,
            tools.swift.map(Path::to_path_buf),
            deadline,
            &cancelled,
        )
    };
    if swift_lint.is_object() {
        crate::native_syntax_confirmation::connect(root, &mut swift_lint, deadline);
    }
    let zig_paths = selected
        .iter()
        .filter(|p| p.ends_with(".zig"))
        .cloned()
        .collect::<BTreeSet<_>>();
    let mut zig_lint = if zig_paths.is_empty() {
        Value::Null
    } else {
        crate::check_zig_scan::observe(
            root,
            &zig_paths,
            tools.zig.map(Path::to_path_buf),
            deadline,
            &cancelled,
        )
    };
    if zig_lint.is_object() {
        crate::native_syntax_confirmation::connect(root, &mut zig_lint, deadline);
        crate::check_zig_scan::refresh(root, &mut zig_lint, deadline);
    }
    let ruby_paths = selected
        .iter()
        .filter(|p| p.ends_with(".rb"))
        .cloned()
        .collect::<BTreeSet<_>>();
    let mut ruby_lint = if ruby_paths.is_empty() {
        Value::Null
    } else {
        crate::check_ruby_scan::observe(
            root,
            &ruby_paths,
            tools.ruby.map(Path::to_path_buf),
            deadline,
            &cancelled,
        )
    };
    if ruby_lint.is_object() {
        crate::native_syntax_confirmation::connect(root, &mut ruby_lint, deadline);
        crate::check_ruby_scan::refresh(root, &mut ruby_lint, deadline);
    }
    let shell_extensions = codeguard_adapters::legacy_registry()
        .ok()
        .and_then(|r| r.languages.into_iter().find(|l| l.id == "shell"))
        .map(|l| l.extensions)
        .unwrap_or_default();
    let shell_paths = selected
        .iter()
        .filter(|p| shell_extensions.iter().any(|e| p.ends_with(e)))
        .cloned()
        .collect::<BTreeSet<_>>();
    let mut shell_lint = if shell_paths.is_empty() {
        Value::Null
    } else {
        crate::check_shell_scan::observe(
            root,
            &shell_paths,
            tools.shellcheck.map(Path::to_path_buf),
            None,
            deadline,
            &cancelled,
        )
    };
    if shell_lint.is_object() {
        crate::check_shell_scan::connect(root, &mut shell_lint, deadline);
        crate::check_shell_scan::refresh(root, &mut shell_lint, deadline);
    }
    let go_paths = selected
        .iter()
        .filter(|p| p.ends_with(".go"))
        .cloned()
        .collect::<BTreeSet<_>>();
    let mut go_syntax = if go_paths.is_empty() {
        Value::Null
    } else {
        crate::check_go_syntax_scan::observe(
            root,
            &go_paths,
            tools.go.map(Path::to_path_buf),
            deadline,
            &cancelled,
        )
    };
    if go_syntax.is_object() {
        crate::native_syntax_confirmation::connect(root, &mut go_syntax, deadline);
        crate::check_go_syntax_scan::refresh(root, &mut go_syntax, deadline);
    }
    let rust_paths = selected
        .iter()
        .filter(|p| p.ends_with(".rs"))
        .cloned()
        .collect::<BTreeSet<_>>();
    let mut rust_syntax = if rust_paths.is_empty() {
        Value::Null
    } else {
        crate::check_rust_syntax_scan::observe(
            root,
            &rust_paths,
            tools.rustfmt.map(Path::to_path_buf),
            deadline,
            &cancelled,
        )
    };
    if rust_syntax.is_object() {
        crate::native_syntax_confirmation::connect(root, &mut rust_syntax, deadline);
    }
    let erlang_paths = selected
        .iter()
        .filter(|p| p.ends_with(".erl") || p.ends_with(".hrl"))
        .cloned()
        .collect::<BTreeSet<_>>();
    let mut erlang_lint = if erlang_paths.is_empty() {
        Value::Null
    } else {
        crate::check_erlang_scan::observe(
            root,
            &erlang_paths,
            tools.erl.map(Path::to_path_buf),
            deadline,
            &cancelled,
        )
    };
    if erlang_lint.is_object() {
        crate::check_erlang_scan::refresh(root, &mut erlang_lint, deadline);
        // 缺工具先由候选分流是否需要确认；选择过工具的失败必须保留原生环境任务。
        if !cfg!(feature = "wasm-precheck")
            || erlang_lint["tool_selection"]["source"] != "not_found"
        {
            crate::native_syntax_confirmation::connect(root, &mut erlang_lint, deadline);
            crate::check_erlang_scan::refresh(root, &mut erlang_lint, deadline);
        }
    }
    // 所选原生入口故障不能以WASM掩盖；确实缺入口才允许候选初检。
    #[cfg(feature = "wasm-precheck")]
    let syntax_selected = selected
        .iter()
        .filter(|p| !p.ends_with(".rs") || rust_syntax["tool_selection"]["source"] == "not_found")
        .filter(|p| {
            !(p.ends_with(".erl") || p.ends_with(".hrl"))
                || erlang_lint["tool_selection"]["source"] == "not_found"
        })
        .cloned()
        .collect::<BTreeSet<_>>();
    #[cfg(feature = "wasm-precheck")]
    let syntax = crate::check_syntax_candidates::observe_selected(
        root,
        &syntax_selected,
        crate::check_syntax_candidates::NativeCoverage {
            node_lint: &node_lint,
            python_lint: &python_lint,
            go_lint: &go_syntax,
            erlang_lint: &erlang_lint,
            kotlin_lint: &kotlin_lint,
            swift_lint: &swift_lint,
            ruby_lint: &ruby_lint,
            zig_lint: &zig_lint,
            rust_targets: &crate::rust_native_syntax_coverage::RustNativeSyntaxCoverage::default(),
            go_tool: None,
        },
        2,
        deadline,
        None,
    );
    #[cfg(not(feature = "wasm-precheck"))]
    let syntax = json!({"status":"not_run","reason":"wasm_feature_not_built","observations":[]});
    let syntax_tasks = crate::syntax_confirmation::persist(root, &syntax, deadline);
    if swift_lint.is_object() {
        crate::check_swift_scan::refresh(root, &mut swift_lint, deadline);
    }
    if zig_lint.is_object() {
        crate::check_zig_scan::refresh(root, &mut zig_lint, deadline);
    }
    if ruby_lint.is_object() {
        crate::check_ruby_scan::refresh(root, &mut ruby_lint, deadline);
    }
    if shell_lint.is_object() {
        crate::check_shell_scan::refresh(root, &mut shell_lint, deadline);
    }
    if go_syntax.is_object() {
        crate::check_go_syntax_scan::refresh(root, &mut go_syntax, deadline);
    }
    let native_unwired: Vec<&String> = selected
        .iter()
        .filter(|p| {
            !p.ends_with(".rs")
                && !p.ends_with(".go")
                && !p.ends_with(".erl")
                && !p.ends_with(".hrl")
                && !shell_paths.contains(*p)
                && !p.ends_with(".rb")
                && !p.ends_with(".zig")
                && !p.ends_with(".py")
                && !p.ends_with(".kt")
                && !p.ends_with(".swift")
                && !is_source(p)
        })
        .collect();
    let recoveries = syntax["observations"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|row| row["recovery_count"].as_u64().unwrap_or(0))
        .sum::<u64>();
    let candidate_count = syntax["observations"].as_array().map_or(0, Vec::len);
    let structures = syntax["observations"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|row| row["structural_observation_count"].as_u64().unwrap_or(0))
        .sum::<u64>();
    let next_action = if recoveries > 0
        || structures > 0
        || syntax["observations"].as_array().is_some_and(|rows| {
            rows.iter().any(|row| {
                row["status"] == "candidate_observed"
                    && row["reason"] == "syntax_recovery_incomplete"
            })
        }) {
        "require_native_lint_confirmation"
    } else if shell_lint["files"].as_array().is_some_and(|files| {
        files.iter().any(|f| {
            f["input_stable"] == true
                && f["native"]["diagnostics"]
                    .as_array()
                    .is_some_and(|d| !d.is_empty())
        })
    }) || [
        &rust_syntax,
        &go_syntax,
        &swift_lint,
        &zig_lint,
        &ruby_lint,
        &erlang_lint,
    ]
    .iter()
    .any(|report| {
        report["files"].as_array().is_some_and(|files| {
            files.iter().any(|f| {
                f["current"] == true
                    && f["native"]["diagnostics"]
                        .as_array()
                        .is_some_and(|d| !d.is_empty())
            })
        })
    }) {
        "repair_native_source"
    } else if unavailable.is_empty()
        && candidate_count > 0
        && syntax["status"] == "observed_partial"
        && syntax["skipped_count"] == 0
        && syntax["unrouted_count"] == 0
        && syntax["observations"].as_array().is_some_and(|rows| {
            rows.iter()
                .all(|r| r["status"] == "candidate_observed" && r["reason"].is_null())
        })
    {
        "recommend_native_lint"
    } else {
        "review_native_results_and_resolve_incomplete_checks"
    };
    let cfquery = syntax["observations"]
        .as_array()
        .is_some_and(|rows| rows.iter().any(|r| r["language"] == "cfquery"));
    let mut feedback = json!({
        "schema_version":if erlang_lint.is_object() || syntax["observations"].as_array().is_some_and(|rows| rows.iter().any(|row|row["language"]=="erlang" && row.get("structural_observations").is_some())) {"0.16.0"}else if syntax["observations"].as_array().is_some_and(|rows| rows.iter().any(|row| row["language"]=="javascript" && row.get("structural_observations").is_some())) {"0.15.0"}else if rust_syntax.is_object(){"0.14.0"}else if cfquery {"0.13.0"}else if go_syntax.is_object(){"0.12.0"}else if shell_lint.is_object(){"0.11.0"}else if ruby_lint.is_object(){"0.10.0"}else if structures > 0 && syntax["observations"].as_array().is_some_and(|rows| rows.iter().any(|row| row["language"] == "go" && row.get("structural_observations").is_some())) {"0.9.0"} else if cfquery || structures > 0 {"0.8.0"} else if zig_lint["schema_version"] == "0.2.0" {"0.7.0"}else if zig_lint.is_object(){"0.6.0"}else if swift_lint["schema_version"] == "0.2.0" {"0.5.0"} else if swift_lint.is_object(){"0.4.0"}else if kotlin_lint.is_object(){"0.3.0"}else{"0.2.0"},"report_type":"hook_fast_feedback",
        "scan_scope":"selected_files","requested_paths":requested,
        "python_lint":python_lint,"node_lint":node_lint,"syntax_candidates":syntax,"syntax_tasks":syntax_tasks,
        "unavailable_files":unavailable,"native_unwired_files":native_unwired,
        "candidate_recovery_count":recoveries,"next_action":next_action,
        "delivery_decision":"not_evaluated","coverage_proven":false
    });
    if rust_syntax.is_object() || cfquery || structures > 0 {
        feedback["candidate_structure_count"] = json!(structures);
    }
    if feedback["schema_version"] == "0.16.0"
        || rust_syntax.is_object()
        || cfquery
        || go_syntax.is_object()
        || shell_lint.is_object()
        || ruby_lint.is_object()
        || structures > 0
        || kotlin_lint.is_object()
        || swift_lint.is_object()
        || zig_lint.is_object()
    {
        feedback["kotlin_lint"] = kotlin_lint;
    }
    if feedback["schema_version"] == "0.16.0"
        || rust_syntax.is_object()
        || cfquery
        || go_syntax.is_object()
        || shell_lint.is_object()
        || ruby_lint.is_object()
        || structures > 0
        || swift_lint.is_object()
        || zig_lint.is_object()
    {
        feedback["swift_lint"] = swift_lint;
    }
    if feedback["schema_version"] == "0.16.0"
        || rust_syntax.is_object()
        || cfquery
        || go_syntax.is_object()
        || shell_lint.is_object()
        || ruby_lint.is_object()
        || structures > 0
        || zig_lint.is_object()
    {
        feedback["zig_lint"] = zig_lint;
    }
    if matches!(
        feedback["schema_version"].as_str(),
        Some("0.15.0" | "0.16.0")
    ) || rust_syntax.is_object()
        || cfquery
        || go_syntax.is_object()
        || shell_lint.is_object()
        || ruby_lint.is_object()
    {
        feedback["ruby_lint"] = ruby_lint;
    }
    if matches!(
        feedback["schema_version"].as_str(),
        Some("0.15.0" | "0.16.0")
    ) || rust_syntax.is_object()
        || cfquery
        || go_syntax.is_object()
        || shell_lint.is_object()
    {
        feedback["shell_lint"] = shell_lint;
    }
    if matches!(
        feedback["schema_version"].as_str(),
        Some("0.15.0" | "0.16.0")
    ) || rust_syntax.is_object()
        || cfquery
        || go_syntax.is_object()
    {
        feedback["go_syntax"] = go_syntax;
    }
    if matches!(
        feedback["schema_version"].as_str(),
        Some("0.15.0" | "0.16.0")
    ) || rust_syntax.is_object()
    {
        feedback["rust_syntax"] = rust_syntax;
    }
    if feedback["schema_version"] == "0.16.0" {
        feedback["erlang_lint"] = erlang_lint;
    }
    feedback
}

fn safe_file(root: &Path, relative: &str) -> bool {
    let mut path = root.to_path_buf();
    for component in Path::new(relative).components() {
        if !matches!(component, std::path::Component::Normal(_)) {
            return false;
        }
        path.push(component);
        if !std::fs::symlink_metadata(&path).is_ok_and(|m| !m.file_type().is_symlink()) {
            return false;
        }
    }
    std::fs::metadata(path).is_ok_and(|m| m.is_file() && m.len() <= 1024 * 1024)
}
