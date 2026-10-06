//! 明确标准上下文的冻结stdin原生Clang观察；不构建或执行用户源码。
use codeguard_runtime::{ProcessSpec, Termination, read_bounded_regular_file, run_process};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, ffi::OsString, path::Path, sync::atomic::AtomicBool, time::Instant};

/// 使用固定Clang21对指定标准执行原生语法观察。
/// 参数为明确工具/语言/标准/冻结源码及共同截止时间；返回有界规则及位置，未解析预处理仅作上下文阻塞。
pub(crate) fn observe(
    tool: &Path,
    language: &str,
    standard: &str,
    source: &[u8],
    deadline: Instant,
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
    let cancelled = AtomicBool::new(false);
    let invoke = |args: Vec<OsString>, stdin| {
        run_process(
            &ProcessSpec {
                executable: executable.clone(),
                args,
                cwd: scratch.path().to_path_buf(),
                env: BTreeMap::new(),
                stdin,
                deadline,
                output_limit_bytes: 64 * 1024,
            },
            &cancelled,
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
        "-Wall",
        "-Wextra",
        "-Wpedantic",
        "-nostdinc",
        "-x",
        language,
        "-",
    ];
    let mut args: Vec<OsString> = flags.into_iter().map(OsString::from).collect();
    args.insert(1, OsString::from(format!("-std={standard}")));
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
    if !output.stdout.is_empty() {
        report["reason"] = json!("clang_report_invalid");
        return report;
    }
    match codeguard_adapters::parse_clang_stdin_sarif(
        &output.stderr,
        source,
        output.termination == Termination::Exited(0),
    ) {
        Ok(rows) => {
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
    report
}
