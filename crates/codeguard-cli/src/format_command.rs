//! 统一代码格式检查与应用命令。
//!
//! 参考各语言原生格式化器（rustfmt/gofmt/clang-format/ruff/prettier…），
//! 提供一致入口，使工程师与智能体写出的代码格式一致：
//!
//! - `codeguard format list`                      列出格式化器档案与接入状态
//! - `codeguard format check <language|all> [path]` 只读检查，不改源码
//! - `codeguard format apply <language|all> [path]` 应用格式化，就地写入
//!
//! 铁律：format 是独立类别，只判风格；不冒充注释或规范合规。

use serde_json::{Value, json};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant};

/// 单文件读取上限，防止格式化器读入无界输入。
const MAX_FILE_BYTES: u64 = 2 * 1024 * 1024;
/// 单次运行扫描文件上限。
const MAX_FILES: usize = 512;

pub fn run(args: &[String]) -> std::process::ExitCode {
    match args.first().map(String::as_str) {
        Some("list") => list(args.get(1..).unwrap_or(&[])),
        Some("check") => execute(args.get(1..).unwrap_or(&[]), Mode::Check),
        Some("apply") => execute(args.get(1..).unwrap_or(&[]), Mode::Apply),
        _ => {
            eprintln!(
                "用法: codeguard format list [--format human|json]\n\
                 用法: codeguard format check <language|all> [path] [--tool NAME=ABS_PATH] [--format human|json]\n\
                 用法: codeguard format apply <language|all> [path] [--tool NAME=ABS_PATH] [--format human|json]"
            );
            std::process::ExitCode::from(2)
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Mode {
    Check,
    Apply,
}

impl Mode {
    fn as_str(self) -> &'static str {
        match self {
            Mode::Check => "check",
            Mode::Apply => "apply",
        }
    }
    fn mutates_sources(self) -> bool {
        matches!(self, Mode::Apply)
    }
}

/// 解析命令行参数。
struct Request {
    selection: String,
    path: PathBuf,
    tools: Vec<(String, String)>,
    json_output: bool,
}

fn parse(args: &[String], default_path: &str) -> Result<Request, String> {
    let mut selection = String::new();
    let mut path = PathBuf::from(default_path);
    let mut tools = Vec::new();
    let mut json_output = false;
    let mut i = 0;
    while i < args.len() {
        // 同时接受 `--flag value` 与 `--flag=value` 两种写法。
        let token = args[i].as_str();
        if let Some(value) = token.strip_prefix("--format=") {
            json_output = value == "json";
            i += 1;
            continue;
        }
        if let Some(value) = token.strip_prefix("--timeout=") {
            if value.is_empty() {
                return Err("--timeout 需要 DURATION".into());
            }
            i += 1;
            continue;
        }
        if let Some(value) = token.strip_prefix("--tool=") {
            let (name, path) = value
                .split_once('=')
                .ok_or("--tool 参数格式应为 NAME=ABS_PATH")?;
            tools.push((name.to_owned(), path.to_owned()));
            i += 1;
            continue;
        }
        match token {
            "--format" | "--output-format" => {
                i += 1;
                if args.get(i).map(String::as_str) == Some("json") {
                    json_output = true;
                }
            }
            "--tool" => {
                i += 1;
                let raw = args.get(i).ok_or("--tool 需要 NAME=ABS_PATH 参数")?;
                let (name, value) = raw
                    .split_once('=')
                    .ok_or("--tool 参数格式应为 NAME=ABS_PATH")?;
                tools.push((name.to_owned(), value.to_owned()));
            }
            "--timeout" => {
                i += 1;
                args.get(i).ok_or("--timeout 需要 DURATION")?;
            }
            other if other.starts_with('-') => return Err(format!("未知参数: {other}")),
            other => {
                if selection.is_empty() {
                    selection = other.to_owned();
                } else if path == PathBuf::from(default_path) {
                    path = PathBuf::from(other);
                } else {
                    return Err(format!("多余的位置参数: {other}"));
                }
            }
        }
        i += 1;
    }
    if selection.is_empty() {
        return Err("需要 <language|all> 选择".into());
    }
    Ok(Request {
        selection,
        path,
        tools,
        json_output,
    })
}

/// 读取内置格式化器档案。
fn profiles() -> Vec<Value> {
    let raw = include_str!("../../../rulepacks/format_profiles.json");
    serde_json::from_str::<Value>(raw).expect("format_profiles.json 应可解析")["languages"]
        .as_array()
        .cloned()
        .unwrap_or_default()
}

fn list(args: &[String]) -> std::process::ExitCode {
    let json_output = args.iter().any(|arg| {
        arg == "json" || arg == "--format=json" || arg == "--output-format=json"
    });
    let rows = profiles();
    if json_output {
        let report = json!({
            "schema_version": "0.1.0",
            "report_type": "format_profile_inventory",
            "category": "format",
            "authority": "repository_profile_only",
            "separation_note": "format 独立于 lint/comments/conventions；格式化不能冒充注释或规范检查",
            "language_count": rows.len(),
            "integrated_count": rows.iter().filter(|r| r["status"] == "integrated").count(),
            "languages": rows,
        });
        println!("{}", serde_json::to_string(&report).unwrap());
    } else {
        println!("Codeguard 格式化器档案（format 类别，仅判风格）");
        println!();
        println!("{:<14} {:<16} {:<10} 说明", "语言", "格式化器", "状态");
        for row in &rows {
            let formatter = row["formatter"].as_str().unwrap_or("-");
            let status = match row["status"].as_str() {
                Some("integrated") => "已接入",
                _ => "未接入",
            };
            println!(
                "{:<14} {:<16} {:<10} {}",
                row["language"].as_str().unwrap_or("-"),
                formatter,
                status,
                row["notes"].as_str().unwrap_or("")
            );
        }
    }
    std::process::ExitCode::SUCCESS
}

fn execute(args: &[String], mode: Mode) -> std::process::ExitCode {
    let request = match parse(args, ".") {
        Ok(request) => request,
        Err(error) => {
            eprintln!("参数错误: {error}");
            return std::process::ExitCode::from(2);
        }
    };
    let started = Instant::now();
    let all = profiles();
    let selected: Vec<Value> = if request.selection == "all" {
        all.iter()
            .filter(|row| row["status"] == "integrated")
            // 路由条目（如 ansible→yaml）在 all 模式下不重复执行：
            // 目标通道已在同一轮被检查，再跑一次会重复格式化同一批文件，
            // 并可能在两侧配置漂移时产生互相矛盾的风格判定。
            .filter(|row| row["routed_to"].is_null())
            .cloned()
            .collect()
    } else {
        // 点名选择：支持路由。ansible 无专属格式化器但其 playbook 是 YAML，
        // 归口 yaml 通道执行，使 `format check ansible` 真正可用而非仅声明缺口。
        let direct: Vec<Value> = all
            .iter()
            .filter(|row| row["language"] == request.selection)
            .cloned()
            .collect();
        if let Some(target) = direct
            .first()
            .and_then(|row| row["routed_to"].as_str())
        {
            let resolved: Vec<Value> = all
                .iter()
                .filter(|row| row["language"] == target && row["status"] == "integrated")
                .cloned()
                .collect();
            if let Some(mut base) = resolved.first().cloned() {
                base["routed_from"] = json!(request.selection);
                base["routed_to"] = json!(target);
                vec![base]
            } else {
                Vec::new()
            }
        } else {
            direct
        }
    };

    if selected.is_empty() {
        let report = json!({
            "schema_version": "0.1.0",
            "report_type": "format_feedback",
            "category": "format",
            "operation": mode.as_str(),
            "selection": request.selection,
            "status": "not_integrated",
            "command_status": "incomplete",
            "delivery_decision": "not_evaluated",
            "exit_code": 3,
            "reason": "format_not_integrated_for_language",
            "next_action": format!(
                "{} 尚未接入原生格式化器；选择已接入语言或为该语言补充 rulepacks/format_profiles.json 档案",
                request.selection
            ),
        });
        emit(&report, request.json_output);
        return std::process::ExitCode::from(3);
    }

    // 选中语言未接入格式化器时必须如实披露，不得因无源文件静默放行。
    let unintegrated: Vec<&str> = selected
        .iter()
        .filter(|row| row["status"] != "integrated")
        .filter_map(|row| row["language"].as_str())
        .collect();
    if !unintegrated.is_empty() {
        let report = json!({
            "schema_version": "0.1.0",
            "report_type": "format_feedback",
            "category": "format",
            "operation": mode.as_str(),
            "selection": request.selection,
            "status": "not_integrated",
            "command_status": "incomplete",
            "delivery_decision": "not_evaluated",
            "exit_code": 3,
            "reason": "format_not_integrated_for_language",
            "not_integrated_languages": unintegrated,
            "next_action": format!(
                "以下语言尚未接入原生格式化器：{}；补充 rulepacks/format_profiles.json 档案后重新检查",
                unintegrated.join("/")
            ),
        });
        emit(&report, request.json_output);
        return std::process::ExitCode::from(3);
    }

    let mut language_results: Vec<Value> = Vec::new();
    let mut total_unformatted: Vec<String> = Vec::new();
    let mut total_reformatted: Vec<String> = Vec::new();
    let mut total_failed: Vec<String> = Vec::new();

    for profile in &selected {
        let language = profile["language"].as_str().unwrap_or("unknown");
        let result = run_language(profile, &request, mode);
        for path in result["unformatted"].as_array().into_iter().flatten() {
            if let Some(p) = path.as_str() {
                total_unformatted.push(p.to_owned());
            }
        }
        for path in result["reformatted"].as_array().into_iter().flatten() {
            if let Some(p) = path.as_str() {
                total_reformatted.push(p.to_owned());
            }
        }
        for key in ["failed", "unparsable"] {
            for path in result[key].as_array().into_iter().flatten() {
                if let Some(p) = path.as_str() {
                    total_failed.push(p.to_owned());
                }
            }
        }
        language_results.push(json!({
            "language": language,
            "formatter": profile["formatter"],
            "routed_from": profile["routed_from"],
            "routed_to": profile["routed_to"],
            "status": result["status"],
            "reason": result["reason"],
            "tool_version": result["tool_version"],
            "tool_digest": result["tool_digest"],
            "config_ref": result["config_ref"],
            "file_count": result["file_count"],
            "unformatted": result["unformatted"],
            "reformatted": result["reformatted"],
            "failed": result["failed"],
            "notes": profile["notes"],
        }));
    }

    let has_unformatted = !total_unformatted.is_empty();
    let has_failed = !total_failed.is_empty();
    // 任一语言未完成（缺工具/无源文件）都不得汇总为 allow。
    let has_incomplete_language = language_results
        .iter()
        .any(|result| result["status"] != "complete");
    let (status, decision, exit_code) = if has_failed {
        ("incomplete", "not_evaluated", 3)
    } else if has_incomplete_language {
        // 语言级未完成：如实披露，不静默放行。
        ("incomplete", "not_evaluated", 3)
    } else if mode == Mode::Check && has_unformatted {
        ("complete", "deny", 1)
    } else if mode == Mode::Apply && !total_reformatted.is_empty() {
        ("reformatted", "incomplete", 3)
    } else {
        ("complete", "allow", 0)
    };

    let report = json!({
        "schema_version": "0.1.0",
        "report_type": "format_feedback",
        "category": "format",
        "operation": mode.as_str(),
        "selection": request.selection,
        "status": status,
        "command_status": if status == "complete" || status == "reformatted" { "complete" } else { "incomplete" },
        "delivery_decision": decision,
        "exit_code": exit_code,
        "authority": "local_unverified",
        "root": request.path.to_string_lossy(),
        "mutates_sources": mode.mutates_sources(),
        "separation_note": "format 只判代码风格；不冒充注释完整性、开发规范或依赖漏洞检查",
        "elapsed_ms": started.elapsed().as_millis() as u64,
        "language_count": language_results.len(),
        "unformatted_count": total_unformatted.len(),
        "reformatted_count": total_reformatted.len(),
        "failed_count": total_failed.len(),
        "unformatted_files": total_unformatted,
        "reformatted_files": total_reformatted,
        "failed_files": total_failed,
        "languages": language_results,
        "next_action": match (mode, has_unformatted, has_failed, !total_reformatted.is_empty()) {
            (_, true, _, _) => "运行 codeguard format apply <language> 统一格式，或按项目原生格式化器手动修正后复检",
            (_, _, true, _) => "修正上述工具失败文件后重试；失败不得视为格式合规",
            (Mode::Apply, false, false, true) => {
                "已就地统一格式；运行 codeguard format check 复检并查看改动差异"
            }
            (Mode::Apply, false, false, false) => "所有文件已符合项目格式规范",
            _ => "格式检查通过",
        },
    });
    emit(&report, request.json_output);
    std::process::ExitCode::from(exit_code as u8)
}

fn run_language(profile: &Value, request: &Request, mode: Mode) -> Value {
    let language = profile["language"].as_str().unwrap_or("unknown");
    let extensions: Vec<&str> = profile["extensions"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|e| e.as_str())
        .collect();

    let mut files = Vec::new();
    collect_files(&request.path, &extensions, &mut files, MAX_FILES);
    files.sort();
    files.dedup();

    let config_ref = profile["config_markers"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|m| m.as_str())
        .map(|m| request.path.join(m))
        .find(|p| p.exists())
        .map(|p| p.file_name().unwrap_or_default().to_string_lossy().to_string())
        .unwrap_or_else(|| "none".to_string());

    if files.is_empty() {
        return json!({
            "status": "no_files",
            "reason": "no_matching_source_files",
            "tool_version": Value::Null,
            "tool_digest": Value::Null,
            "config_ref": config_ref,
            "file_count": 0,
            "unformatted": [],
            "reformatted": [],
            "failed": [],
        });
    }

    // 解析格式化器可执行文件：显式 --tool 优先，其次同名 PATH。
    // 显式路径不存在时不得回退 PATH，避免绕过用户指定的工具身份。
    let tool_key = profile["tool_key"].as_str().unwrap_or("");
    let explicit = request
        .tools
        .iter()
        .find(|(name, _)| name == tool_key)
        .map(|(_, value)| PathBuf::from(value));
    let tool = match explicit {
        Some(path) => path.is_file().then_some(path),
        None => which(tool_key),
    };

    let Some(tool) = tool else {
        return json!({
            "status": "incomplete",
            "reason": "formatter_not_found",
            "tool_version": Value::Null,
            "tool_digest": Value::Null,
            "config_ref": config_ref,
            "file_count": files.len(),
            "unformatted": [],
            "reformatted": [],
            "failed": [],
        });
    };

    // 冻结工具版本与字节身份，避免工具升级后沿用旧结论。
    let version = Command::new(&tool)
        .arg("--version")
        .output()
        .ok()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_default();
    let tool_digest = file_digest(&tool)
        .map(Value::String)
        .unwrap_or(Value::Null);

    let argv_template: Vec<String> = profile[match mode {
        Mode::Check => "check_argv",
        Mode::Apply => "apply_argv",
    }]
    .as_array()
    .into_iter()
    .flatten()
    .filter_map(|a| a.as_str())
    .map(str::to_owned)
    .collect();

    // 读取该语言声明的「不合规」退出码集合；缺省为 B 类标准的 [1]。
    // 档案显式声明是必要的：terraform fmt -check=3、perltidy -ast=2、php-cs-fixer=8。
    let unformatted_codes: Vec<i32> = profile["unformatted_exit_codes"]
        .as_array()
        .map(|codes| codes.iter().filter_map(Value::as_i64).map(|c| c as i32).collect())
        .filter(|codes: &Vec<i32>| !codes.is_empty())
        .unwrap_or_else(|| vec![1]);

    let mut unformatted = Vec::new();
    let mut unparsable = Vec::new();
    let mut reformatted = Vec::new();
    let mut failed = Vec::new();

    for file in &files {
        if let Ok(meta) = std::fs::metadata(file) {
            if meta.len() > MAX_FILE_BYTES {
                failed.push(file.to_string_lossy().to_string());
                continue;
            }
        } else {
            failed.push(file.to_string_lossy().to_string());
            continue;
        }
        let argv: Vec<String> = argv_template
            .iter()
            .map(|token| {
                token
                    .replace("{file}", &file.to_string_lossy())
                    .replace("{edition}", "2021")
                    .replace("{standard}", "c11")
            })
            .collect();
        let output = Command::new(&tool).args(&argv).output();
        match output {
            Ok(result) if result.status.success() => {
                // A 类语义：退出 0 但 stdout 列出文件（gofmt -l / prettier --list-different）。
                let stdout = String::from_utf8_lossy(&result.stdout);
                if mode == Mode::Check && !stdout.trim().is_empty() {
                    unformatted.push(file.to_string_lossy().to_string());
                } else if mode == Mode::Check && detect_unparsable(&tool, file) {
                    // 工具对无法解析的源文件会静默退 0（clang-format 的已知行为：
                    // 解析失败时原样输出并返回成功）。这会把「无法检查」误报成
                    // 「格式合规」，必须用第二遍探测把它降级为未完成。
                    unparsable.push(file.to_string_lossy().to_string());
                } else if mode == Mode::Apply {
                    reformatted.push(file.to_string_lossy().to_string());
                }
            }
            // B 类语义：退出码表示不合规。多数工具用 1，但部分工具（terraform fmt -check=3、
            // perltidy -ast=2、php-cs-fixer dry-run=8）用其它非零码，必须按档案声明的码集合判定，
            // 否则真实的不合规会被误报为工具故障。
            Ok(result) if mode == Mode::Check
                && result
                    .status
                    .code()
                    .is_some_and(|code| unformatted_codes.contains(&code)) =>
            {
                unformatted.push(file.to_string_lossy().to_string());
            }
            Ok(_) => failed.push(file.to_string_lossy().to_string()),
            Err(_) => failed.push(file.to_string_lossy().to_string()),
        }
    }

    json!({
        "status": if failed.is_empty() && unparsable.is_empty() { "complete" } else { "incomplete" },
        "reason": if !unparsable.is_empty() {
            json!("source_not_parsable_by_formatter")
        } else if failed.is_empty() {
            Value::Null
        } else {
            json!("formatter_execution_failed")
        },
        "tool_version": version,
        "tool_digest": tool_digest,
        "config_ref": config_ref,
        "file_count": files.len(),
        "unformatted": unformatted,
        "reformatted": reformatted,
        "failed": failed,
        "unparsable": unparsable,
    })
}

/// 探测格式化器是否**静默放行**了无法解析的源文件。
///
/// 已知问题：clang-format 在遇到解析失败的源文件时，会原样输出内容并返回成功退出码，
/// 于是「无法检查」被误报为「格式合规」。它只在 `--output-replacements-xml` 的
/// `incomplete_format` 属性里如实标注 `true`。只读检查若不额外探测，
/// 损坏的源文件（包括 Metal 着色器、CUDA 内核等借用的 C 系方言）会被静默放行。
///
/// 这里对已用 C 系解析器（clang-format）的语言做第二遍探测：
/// 用与检查相同的输入跑一次 `--output-replacements-xml`，读 `incomplete_format`。
/// 只有明确标注 `incomplete_format='true'` 才判定为不可解析；其它工具不适用此探测，
/// 返回 false 让它们沿用自身退出码语义。
fn detect_unparsable(tool: &Path, file: &Path) -> bool {
    let name = tool
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();
    if !name.contains("clang-format") {
        return false;
    }
    let probe = Command::new(tool)
        .arg("--output-replacements-xml")
        .arg(file)
        .output();
    match probe {
        Ok(result) => {
            let xml = String::from_utf8_lossy(&result.stdout);
            xml.contains("incomplete_format='true'") || xml.contains("incomplete_format=\"true\"")
        }
        Err(_) => false,
    }
}

fn collect_files(dir: &Path, extensions: &[&str], out: &mut Vec<PathBuf>, limit: usize) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        // 允许直接传单文件。
        if dir.is_file() {
            if let Some(ext) = dir.extension().and_then(|e| e.to_str()) {
                if extensions
                    .iter()
                    .any(|allowed| allowed.trim_start_matches('.').eq_ignore_ascii_case(ext))
                {
                    out.push(dir.to_path_buf());
                }
            }
        }
        return;
    };
    for entry in entries.flatten() {
        if out.len() >= limit {
            return;
        }
        let path = entry.path();
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if name.starts_with('.') {
            continue;
        }
        if path.is_dir() {
            if matches!(name.as_ref(), "node_modules" | "target" | "dist" | "build" | ".git") {
                continue;
            }
            collect_files(&path, extensions, out, limit);
        } else if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            if extensions
                .iter()
                .any(|allowed| allowed.trim_start_matches('.').eq_ignore_ascii_case(ext))
            {
                out.push(path);
            }
        }
    }
}

/// 解析 PATH 中的可执行文件。
fn which(name: &str) -> Option<PathBuf> {
    if name.is_empty() {
        return None;
    }
    let path_var = std::env::var_os("PATH")?;
    for dir in std::env::split_paths(&path_var) {
        let candidate = dir.join(name);
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    None
}

fn file_digest(path: &Path) -> Option<String> {
    let bytes = std::fs::read(path).ok()?;
    Some(stable_digest(&bytes))
}

fn stable_digest(bytes: &[u8]) -> String {
    // 轻量 FNV-1a 派生的稳定摘要；仅用于同次运行内的身份比对，
    // 不作为密码学强度主张。
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    format!("{hash:016x}{}", bytes.len())
}

fn emit(report: &Value, json_output: bool) {
    if json_output {
        println!("{}", serde_json::to_string(report).unwrap());
        return;
    }
    println!(
        "codeguard format {} · {} · {}",
        report["operation"].as_str().unwrap_or("-"),
        report["status"].as_str().unwrap_or("-"),
        report["delivery_decision"].as_str().unwrap_or("-")
    );
    for language in report["languages"].as_array().into_iter().flatten() {
        println!(
            "  {:<12} {:<14} 文件 {:<5} 不合规 {:<5} 已格式化 {:<5} 失败 {}",
            language["language"].as_str().unwrap_or("-"),
            language["formatter"].as_str().unwrap_or("-"),
            language["file_count"],
            language["unformatted"].as_array().map(Vec::len).unwrap_or(0),
            language["reformatted"].as_array().map(Vec::len).unwrap_or(0),
            language["failed"].as_array().map(Vec::len).unwrap_or(0),
        );
    }
    for file in report["unformatted_files"].as_array().into_iter().flatten() {
        if let Some(f) = file.as_str() {
            println!("  不合规: {f}");
        }
    }
    println!("  下一步: {}", report["next_action"].as_str().unwrap_or("-"));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn profiles_cover_all_registered_languages() {
        let rows = profiles();
        assert_eq!(rows.len(), 57, "格式化档案应覆盖 57 语言");
    }

    #[test]
    fn integrated_profiles_have_formatter_and_argv() {
        for row in profiles().iter().filter(|r| r["status"] == "integrated") {
            assert!(
                row["formatter"].as_str().is_some_and(|f| !f.is_empty()),
                "{} 应有格式化器",
                row["language"]
            );
            assert!(
                row["check_argv"].as_array().is_some_and(|a| !a.is_empty()),
                "{} 应有只读检查参数",
                row["language"]
            );
            assert!(
                row["apply_argv"].as_array().is_some_and(|a| !a.is_empty()),
                "{} 应有应用参数",
                row["language"]
            );
            assert!(
                row["extensions"].as_array().is_some_and(|a| !a.is_empty()),
                "{} 应有扩展名",
                row["language"]
            );
        }
    }

    #[test]
    fn not_integrated_profiles_have_no_silent_pass() {
        for row in profiles().iter().filter(|r| r["status"] == "not_integrated") {
            assert!(
                row["formatter"].is_null(),
                "{} 未接入时不应声明格式化器",
                row["language"]
            );
            assert!(
                row["check_argv"].is_null() && row["apply_argv"].is_null(),
                "{} 未接入时不应声明检查/应用参数",
                row["language"]
            );
            let notes = row["notes"].as_str().unwrap_or("");
            assert!(
                notes.len() >= 20,
                "{} 未接入说明应写明具体缺口理由: {notes}",
                row["language"]
            );
        }
    }

    #[test]
    fn every_profile_declares_unformatted_exit_codes() {
        // 退出码契约：多数工具用 1，terraform=3、perltidy=2、php-cs-fixer=8。
        // 缺省为 [1]；档案显式声明非标准码，避免真实不合规被误判为工具故障。
        for row in profiles().iter().filter(|r| r["status"] == "integrated") {
            let codes: Vec<i64> = row["unformatted_exit_codes"]
                .as_array()
                .map(|c| c.iter().filter_map(Value::as_i64).collect())
                .unwrap_or_default();
            assert!(
                !codes.is_empty(),
                "{} 应声明 unformatted_exit_codes",
                row["language"]
            );
            assert!(
                codes.iter().all(|code| *code > 0),
                "{} 的不合规退出码必须为正数: {codes:?}",
                row["language"]
            );
            assert!(
                !codes.contains(&0),
                "{} 不得把退出 0 声明为不合规",
                row["language"]
            );
        }
    }

    #[test]
    fn non_standard_exit_codes_are_justified_in_notes() {
        // 非标准不合规退出码必须在 notes 中说明来源，否则是无法审计的特例。
        for row in profiles().iter().filter(|r| r["status"] == "integrated") {
            let codes: Vec<i64> = row["unformatted_exit_codes"]
                .as_array()
                .map(|c| c.iter().filter_map(Value::as_i64).collect())
                .unwrap_or_default();
            if codes.iter().any(|code| *code != 1) {
                let notes = row["notes"].as_str().unwrap_or("");
                assert!(
                    notes.contains("退出") || notes.contains("exit"),
                    "{} 使用非标准不合规退出码 {codes:?}，notes 应说明实测依据",
                    row["language"]
                );
            }
        }
    }

    #[test]
    fn every_integrated_profile_has_file_placeholder() {
        for row in profiles().iter().filter(|r| r["status"] == "integrated") {
            for field in ["check_argv", "apply_argv"] {
                let argv: Vec<&str> = row[field]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter_map(Value::as_str)
                    .collect();
                assert!(
                    argv.iter().any(|token| token.contains("{file}")),
                    "{} 的 {field} 必须含 {{file}} 占位",
                    row["language"]
                );
            }
            let extensions = row["extensions"].as_array().cloned().unwrap_or_default();
            assert!(
                !extensions.is_empty(),
                "{} 已接入时必须声明扩展名",
                row["language"]
            );
            assert!(
                extensions
                    .iter()
                    .filter_map(Value::as_str)
                    .all(|ext| ext.starts_with('.')),
                "{} 扩展名必须带前导点",
                row["language"]
            );
        }
    }

    #[test]
    fn profile_declares_format_category_separation() {
        let raw = include_str!("../../../rulepacks/format_profiles.json");
        let doc: Value = serde_json::from_str(raw).unwrap();
        assert_eq!(doc["category"], "format");
        assert!(
            doc["separation_note"].as_str().is_some_and(|n| n.contains("冒充")),
            "档案应声明 format 不能冒充注释或规范检查"
        );
        assert_eq!(doc["qualification"], "not_granted");
    }

    #[test]
    fn parse_rejects_unknown_arguments() {
        assert!(parse(&["rust".into(), "--bogus".into()], ".").is_err());
    }

    #[test]
    fn parse_accepts_tool_binding() {
        let request = parse(
            &[
                "rust".into(),
                "/tmp/x".into(),
                "--tool".into(),
                "rustfmt=/abs/rustfmt".into(),
            ],
            ".",
        )
        .expect("应解析成功");
        assert_eq!(request.selection, "rust");
        assert_eq!(request.tools.len(), 1);
        assert_eq!(request.tools[0].0, "rustfmt");
    }

    #[test]
    fn collect_files_respects_extension_filter() {
        let dir = std::env::temp_dir().join(format!("cg-format-collect-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("a.rs"), "fn main() {}").unwrap();
        std::fs::write(dir.join("b.txt"), "not rust").unwrap();
        let mut files = Vec::new();
        collect_files(&dir, &[".rs"], &mut files, 16);
        assert_eq!(files.len(), 1, "只应收集 .rs 文件");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn digest_is_stable_and_input_sensitive() {
        let a = stable_digest(b"payload");
        let b = stable_digest(b"payload");
        assert_eq!(a, b, "同一输入摘要应稳定");
        assert_ne!(a, stable_digest(b"other"), "不同输入摘要应不同");
        assert!(!a.is_empty(), "摘要不得为空");
    }
}
#[cfg(test)]
mod format_selection_tests {
    use super::profiles;
    use serde_json::Value;

    /// Java 选型必须说明为何不用 spotless / palantir-java-format。
    /// 这不是偏好问题：codeguard 以 {file} 逐文件驱动，需要单文件可判定的独立 CLI。
    #[test]
    fn java_records_formatter_selection_rationale() {
        let java = profiles()
            .into_iter()
            .find(|row| row["language"] == "java")
            .expect("应有 java 档案");
        assert_eq!(java["status"], "integrated");
        assert_eq!(java["formatter"], "google-java-format");
        let notes = java["notes"].as_str().unwrap_or("");
        assert!(
            notes.contains("spotless"),
            "Java 选型说明应解释为何不用 spotless"
        );
        assert!(
            notes.contains("palantir"),
            "Java 选型说明应解释 palantir-java-format 的关系"
        );
        assert!(
            notes.contains("逐文件") || notes.contains("单文件"),
            "Java 选型说明应点明逐文件调用契约这一根本约束"
        );
    }

    /// 已接入语言不得声称有非标准退出码却不声明。
    #[test]
    fn integrated_languages_never_silently_claim_standard_codes() {
        for row in profiles().iter().filter(|r| r["status"] == "integrated") {
            let codes: Vec<i64> = row["unformatted_exit_codes"]
                .as_array()
                .map(|c| c.iter().filter_map(Value::as_i64).collect())
                .unwrap_or_default();
            assert!(
                !codes.is_empty(),
                "{} 必须显式声明不合规退出码",
                row["language"]
            );
        }
    }

    /// wrapper 型档案（解释器 + 内联脚本）必须能对单文件判定。
    #[test]
    fn interpreter_wrapper_profiles_target_single_file() {
        // 解释器 wrapper 模式：check_argv 形如 [解释器, -e, '<脚本>', '{file}']
        for row in profiles().iter().filter(|r| r["status"] == "integrated") {
            let argv: Vec<String> = row["check_argv"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(|v| v.as_str().map(str::to_owned))
                .collect();
            let is_wrapper = argv.iter().any(|a| a == "-e" || a == "-Command" || a == "-c");
            if is_wrapper {
                // {file} 可以是独立 argv，也可以嵌在解释器脚本字符串内部
                // （例如 pwsh -Command "... $f='{file}' ..."）。两种都必须能替换。
                let has_file = argv
                    .iter()
                    .any(|token| token.contains("{file}"));
                assert!(
                    has_file,
                    "{} 的 wrapper 脚本必须能收到 {{file}} 参数",
                    row["language"]
                );
            }
        }
    }
}
