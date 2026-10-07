//! 明确标准上下文的冻结stdin原生Clang观察；不构建或执行用户源码。
use codeguard_runtime::{ProcessSpec, Termination, read_bounded_regular_file, run_process};
use crate::native_clang_profile::NativeClangProfile;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    ffi::OsString,
    path::Path,
    sync::atomic::AtomicBool,
    time::Instant,
};

/// 使用固定Clang21对指定标准执行原生语法观察。
/// 参数为明确工具/语言/标准/冻结源码及共同截止时间；返回有界规则及位置，未解析预处理仅作上下文阻塞。
pub(crate) fn observe(
    tool: &Path,
    language: &str,
    standard: &str,
    source: &[u8],
    deadline: Instant,
) -> Value {
    observe_with_cancellation(
        tool,
        language,
        standard,
        source,
        deadline,
        &AtomicBool::new(false),
    )
}

/// 对固定标准源码执行共享取消令牌的原生观察；参数与observe一致，额外令牌贯穿两个进程阶段。
pub(crate) fn observe_with_cancellation(
    tool: &Path,
    language: &str,
    standard: &str,
    source: &[u8],
    deadline: Instant,
    cancelled: &AtomicBool,
) -> Value {
    observe_profile(
        tool,
        language,
        standard,
        source,
        deadline,
        cancelled,
        NativeClangProfile::Syntax,
    )
}

/// 使用同一版本/制品/输入/报告边界执行文档警告档案；参数同语法观察，返回原生局部结果。
pub(crate) fn observe_documentation(
    tool: &Path,
    language: &str,
    standard: &str,
    source: &[u8],
    deadline: Instant,
    cancelled: &AtomicBool,
) -> Value {
    observe_profile(
        tool,
        language,
        standard,
        source,
        deadline,
        cancelled,
        NativeClangProfile::Documentation,
    )
}

fn observe_profile(
    tool: &Path,
    language: &str,
    standard: &str,
    source: &[u8],
    deadline: Instant,
    cancelled: &AtomicBool,
    profile: NativeClangProfile,
) -> Value {
    let mut report = json!({"status":"incomplete","reason":"clang_tool_unavailable","version":null,"tool_sha256":null,"diagnostics":[]});
    if Instant::now() >= deadline {
        report["reason"] = json!("clang_execution_incomplete");
        return report;
    }
    if source.len() > 1024 * 1024 || std::str::from_utf8(source).is_err() {
        report["reason"] = json!("clang_source_invalid");
        return report;
    }
    // 预处理会影响头文件、宏与分支；尚无编译数据库时不能把缺上下文报成源码错误。
    if codeguard_adapters::has_c_family_preprocessor_directive(source, language != "c") {
        report["reason"] = json!("clang_preprocessor_context_unresolved");
        return report;
    }
    let Ok(executable) = tool.canonicalize() else {
        return report;
    };
    let Ok(bytes) = read_bounded_regular_file(&executable, 256 * 1024 * 1024) else {
        return report;
    };
    let sha = format!("{:x}", Sha256::digest(bytes));
    report["tool_sha256"] = json!(sha);
    let current = || {
        tool.canonicalize().ok().as_deref() == Some(executable.as_path())
            && read_bounded_regular_file(&executable, 256 * 1024 * 1024)
                .is_ok_and(|b| format!("{:x}", Sha256::digest(b)) == sha)
    };
    let Some(scratch) = crate::doctor_scratch::DoctorScratch::create("clang-syntax") else {
        report["reason"] = json!("clang_scratch_unavailable");
        return report;
    };
    let invoke = |args: Vec<OsString>, stdin| {
        run_process(
            &ProcessSpec {
                executable: executable.clone(),
                args,
                cwd: scratch.path().to_path_buf(),
                env: BTreeMap::new(),
                stdin,
                deadline,
                output_limit_bytes: if matches!(
                    profile,
                    NativeClangProfile::DocumentationStructure
                        | NativeClangProfile::DocumentationPlaceholders
                ) {
                    8 * 1024 * 1024
                } else {
                    64 * 1024
                },
            },
            cancelled,
        )
    };
    let version = invoke(vec![OsString::from("--version")], None);
    if Instant::now() >= deadline {
        report["reason"] = json!("clang_execution_incomplete");
        return report;
    }
    if !current() {
        report["reason"] = json!("clang_tool_changed");
        return report;
    }
    if version.termination != Termination::Exited(0)
        || !version.stderr.is_empty()
        || !version
            .stdout
            .starts_with(b"Apple clang version 21.0.0 (clang-2100.3.34.2)\n")
    {
        report["reason"] = json!("clang_version_unverified");
        return report;
    }
    report["version"] = json!("Apple clang version 21.0.0 (clang-2100.3.34.2)");
    let language = if language == "c" { "c" } else { "c++" };
    let flags = [
        "--no-default-config",
        "-fsyntax-only",
        "-fno-color-diagnostics",
        "-fno-caret-diagnostics",
        "-fdiagnostics-format=sarif",
        "-Wno-sarif-format-unstable",
        "-nostdinc",
        "-x",
        language,
        "-",
    ];
    let mut args: Vec<OsString> = flags.into_iter().map(OsString::from).collect();
    args.insert(1, OsString::from(format!("-std={standard}")));
    args.splice(7..7, profile.warning_flags().iter().map(OsString::from));
    if matches!(
        profile,
        NativeClangProfile::DocumentationStructure | NativeClangProfile::DocumentationPlaceholders
    ) {
        args.splice(
            1..1,
            [OsString::from("-Xclang"), OsString::from("-ast-dump=json")],
        );
    }
    let output = invoke(args, Some(source.to_vec()));
    if Instant::now() >= deadline {
        report["reason"] = json!("clang_execution_incomplete");
        return report;
    }
    if !current() {
        report["reason"] = json!("clang_tool_changed");
        return report;
    }
    if !matches!(output.termination, Termination::Exited(0 | 1)) {
        report["reason"] = json!("clang_execution_incomplete");
        return report;
    }
    if !matches!(
        profile,
        NativeClangProfile::DocumentationStructure | NativeClangProfile::DocumentationPlaceholders
    ) && !output.stdout.is_empty()
    {
        report["reason"] = json!("clang_report_invalid");
        return report;
    }
    match codeguard_adapters::parse_clang_stdin_sarif(
        &output.stderr,
        source,
        output.termination == Termination::Exited(0),
    ) {
        Ok(mut rows) => {
            if matches!(
                profile,
                NativeClangProfile::DocumentationStructure
                    | NativeClangProfile::DocumentationPlaceholders
            ) {
                report["structure_raw_diagnostic_count"] = json!(rows.len());
                // AST输出可触发Clang重新读取注释；按脱敏规则/行列/等级归并并保留原条数。
                let mut seen = BTreeSet::new();
                rows.retain(|row| seen.insert(row.to_string()));
            }
            report["status"] = json!(if rows.is_empty() {
                "completed"
            } else {
                "diagnostics_observed"
            });
            report["reason"] = json!(if rows.is_empty() {
                "clang_native_no_diagnostics"
            } else {
                "clang_native_diagnostics"
            });
            report["diagnostics"] = json!(rows);
        }
        Err(reason) => report["reason"] = json!(reason),
    }
    if matches!(
        profile,
        NativeClangProfile::DocumentationStructure | NativeClangProfile::DocumentationPlaceholders
    ) && matches!(
        report["status"].as_str(),
        Some("completed" | "diagnostics_observed")
    ) && output.termination == Termination::Exited(0)
        && !report["diagnostics"]
            .as_array()
            .into_iter()
            .flatten()
            .any(|row| row["level"] == "error")
    {
        if matches!(profile, NativeClangProfile::DocumentationPlaceholders) {
            report["placeholder_observation"] =
                match codeguard_adapters::parse_clang_documentation_placeholders(
                    &output.stdout,
                    source,
                ) {
                    Ok(observation) => {
                        json!({"status":"observed","reason":"clang_placeholder_observed","observation":observation})
                    }
                    Err(_) => {
                        json!({"status":"incomplete","reason":"clang_placeholder_report_invalid","observation":null})
                    }
                };
        }
        report["structure_observation"] = match codeguard_adapters::parse_clang_documentation_ast(
            &output.stdout,
            source,
        ) {
            Ok(observation) => {
                json!({"status":"observed","reason":"clang_structure_observed","observation":observation})
            }
            Err(_) => {
                json!({"status":"incomplete","reason":"clang_structure_report_invalid","observation":null})
            }
        };
    }
    report
}

/// 同一冻结stdin原生扫描采集警告与文档结构；参数为固定工具/标准/截止时间和取消令牌。
/// 返回原警告及独立结构对象；工具失稳、超时或原生错误均不提供有效结构。
pub(crate) fn observe_documentation_with_structure(
    tool: &Path,
    language: &str,
    standard: &str,
    source: &[u8],
    deadline: Instant,
    cancelled: &AtomicBool,
) -> (Value, Value) {
    let (native, structure, _) = observe_documentation_with_placeholders(
        tool, language, standard, source, deadline, cancelled, false,
    );
    (native, structure)
}

/// 同次原生扫描的独立占位策略观察；enable为false时不执行新增解析，不改变历史消费协议。
pub(crate) fn observe_documentation_with_placeholders(
    tool: &Path,
    language: &str,
    standard: &str,
    source: &[u8],
    deadline: Instant,
    cancelled: &AtomicBool,
    enable: bool,
) -> (Value, Value, Value) {
    let mut native = observe_profile(
        tool,
        language,
        standard,
        source,
        deadline,
        cancelled,
        if enable {
            NativeClangProfile::DocumentationPlaceholders
        } else {
            NativeClangProfile::DocumentationStructure
        },
    );
    let mut structure=native.as_object_mut().and_then(|n|n.remove("structure_observation")).unwrap_or(json!({"status":"incomplete","reason":"clang_structure_unavailable","observation":null}));
    structure["native_raw_diagnostic_count"] = native
        .as_object_mut()
        .and_then(|n| n.remove("structure_raw_diagnostic_count"))
        .unwrap_or(Value::Null);
    let placeholder = native.as_object_mut().and_then(|n| n.remove("placeholder_observation")).unwrap_or(json!({"status":"incomplete","reason":"clang_placeholder_unavailable","observation":null}));
    (native, structure, placeholder)
}
