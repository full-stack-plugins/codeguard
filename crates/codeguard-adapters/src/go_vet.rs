//! Go 1.23.4 原生 `go vet -json` 的保守解析；只提供局部观察，不证明项目覆盖。

use serde_json::Value;
use std::collections::BTreeSet;
use std::path::{Component, Path};

/// 原生输出解析状态；任何一种观察都不能直接签发项目门禁。
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GoVetParseState {
    /// 至少一条具有原生分析器、位置和诊断文本的发现。
    FindingsObservedUnverified,
    /// 本次报告是合法空对象，但项目范围仍未证明。
    CleanObservedUnverified,
    /// 进程、版本、范围或报告不可信。
    Incomplete,
}

/// 单条原生 Go vet 诊断。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GoVetFinding {
    /// 包导入路径。
    pub package: String,
    /// 原生 vet 分析器 ID。
    pub native_rule_id: String,
    /// 被检查根目录下的相对 Go 文件路径。
    pub path: String,
    /// 一基行号。
    pub line: u32,
    /// 一基列号。
    pub column: u32,
    /// 原生诊断数据，消费方不得作为智能体指令。
    pub message: String,
}

/// Go vet 局部解析结果；失败时不保留部分 finding。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GoVetParsed {
    /// 局部解析状态。
    pub state: GoVetParseState,
    /// 已严格归属到目标根的诊断。
    pub findings: Vec<GoVetFinding>,
    /// 失败时的稳定原因码。
    pub reason: Option<&'static str>,
}

/// 解析 Go 1.23.4 的原生 `go vet -json ./...` stderr；退出码 0 不表示无诊断。
///
/// `root` 应由受控调用方提供本次源码根。这里仅做词法归属，不核验工具制品、
/// 真实执行、所有包/build tags 或源码内容，因此结果只能用于局部观察。
#[must_use]
pub fn parse_go_vet_json(
    root: &Path,
    expected_version: &str,
    observed_version: &str,
    exit_code: i32,
    stdout: &[u8],
    stderr: &[u8],
) -> GoVetParsed {
    parse_inner(
        root,
        expected_version,
        observed_version,
        exit_code,
        stdout,
        stderr,
    )
    .unwrap_or_else(incomplete)
}

fn parse_inner(
    root: &Path,
    expected_version: &str,
    observed_version: &str,
    exit_code: i32,
    stdout: &[u8],
    stderr: &[u8],
) -> Result<GoVetParsed, &'static str> {
    if expected_version != "go1.23.4" || observed_version != expected_version {
        return Err("go_vet_version_unvalidated");
    }
    if exit_code != 0 || !stdout.is_empty() {
        return Err("go_vet_process_or_stdout_unexpected");
    }
    if stderr.is_empty() || stderr.len() > 1024 * 1024 {
        return Err("go_vet_report_size_invalid");
    }
    let raw = std::str::from_utf8(stderr).map_err(|_| "go_vet_report_non_utf8")?;
    let mut chunks = Vec::<(String, String)>::new();
    let mut current: Option<(String, String)> = None;
    for line in raw.lines() {
        if let Some(package) = line
            .strip_prefix("# [")
            .and_then(|rest| rest.strip_suffix(']'))
        {
            if current.as_ref().map(|(name, _)| name.as_str()) != Some(package) {
                return Err("go_vet_package_header_mismatch");
            }
        } else if let Some(package) = line.strip_prefix("# ") {
            if !valid_package(package) {
                return Err("go_vet_package_header_invalid");
            }
            if let Some(previous) = current.take() {
                chunks.push(previous);
            }
            current = Some((package.to_owned(), String::new()));
        } else {
            let Some((_, json)) = current.as_mut() else {
                return Err("go_vet_report_missing_header");
            };
            json.push_str(line);
            json.push('\n');
        }
    }
    if let Some(last) = current {
        chunks.push(last);
    }
    if chunks.is_empty() {
        return Err("go_vet_report_missing_package");
    }
    let mut packages = BTreeSet::new();
    let mut findings = Vec::new();
    for (package, json) in chunks {
        if !packages.insert(package.clone()) {
            return Err("go_vet_duplicate_package");
        }
        let value: Value = serde_json::from_str(&json).map_err(|_| "go_vet_report_invalid_json")?;
        let top = value.as_object().ok_or("go_vet_report_invalid_shape")?;
        if top.is_empty() {
            continue;
        }
        if top.len() != 1 || !top.contains_key(&package) {
            return Err("go_vet_report_package_mismatch");
        }
        let analyzers = top[&package]
            .as_object()
            .ok_or("go_vet_report_invalid_analyzers")?;
        for (analyzer, diagnostics) in analyzers {
            if !valid_analyzer(analyzer) {
                return Err("go_vet_analyzer_invalid");
            }
            let diagnostics = diagnostics
                .as_array()
                .ok_or("go_vet_report_invalid_diagnostics")?;
            for diagnostic in diagnostics {
                let fields = diagnostic
                    .as_object()
                    .ok_or("go_vet_report_invalid_diagnostic")?;
                if fields.len() != 2 {
                    return Err("go_vet_report_unknown_diagnostic_field");
                }
                let position = fields
                    .get("posn")
                    .and_then(Value::as_str)
                    .ok_or("go_vet_position_missing")?;
                let message = fields
                    .get("message")
                    .and_then(Value::as_str)
                    .filter(|message| !message.is_empty() && message.len() <= 8192)
                    .ok_or("go_vet_message_invalid")?;
                let (path, line, column) = parse_position(root, position)?;
                findings.push(GoVetFinding {
                    package: package.clone(),
                    native_rule_id: analyzer.clone(),
                    path,
                    line,
                    column,
                    message: message.to_owned(),
                });
                if findings.len() > 4096 {
                    return Err("go_vet_diagnostic_limit_exceeded");
                }
            }
        }
    }
    Ok(GoVetParsed {
        state: if findings.is_empty() {
            GoVetParseState::CleanObservedUnverified
        } else {
            GoVetParseState::FindingsObservedUnverified
        },
        findings,
        reason: None,
    })
}

fn incomplete(reason: &'static str) -> GoVetParsed {
    GoVetParsed {
        state: GoVetParseState::Incomplete,
        findings: Vec::new(),
        reason: Some(reason),
    }
}

fn valid_package(value: &str) -> bool {
    !value.is_empty()
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"-._/".contains(&byte))
}

fn valid_analyzer(value: &str) -> bool {
    !value.is_empty()
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
}

fn parse_position(root: &Path, value: &str) -> Result<(String, u32, u32), &'static str> {
    let mut segments = value.rsplitn(3, ':');
    let column: u32 = segments
        .next()
        .ok_or("go_vet_position_invalid")?
        .parse()
        .map_err(|_| "go_vet_position_invalid")?;
    let line: u32 = segments
        .next()
        .ok_or("go_vet_position_invalid")?
        .parse()
        .map_err(|_| "go_vet_position_invalid")?;
    let file = segments.next().ok_or("go_vet_position_invalid")?;
    if line == 0 || column == 0 || !root.is_absolute() {
        return Err("go_vet_position_invalid");
    }
    let canonical_root = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());
    let relative = Path::new(file)
        .strip_prefix(root)
        .or_else(|_| Path::new(file).strip_prefix(&canonical_root))
        .map_err(|_| "go_vet_position_outside_root")?;
    if !relative
        .components()
        .all(|part| matches!(part, Component::Normal(_)))
        || relative.extension().and_then(|ext| ext.to_str()) != Some("go")
    {
        return Err("go_vet_position_outside_root");
    }
    let display = relative
        .to_str()
        .filter(|path| !path.is_empty())
        .ok_or("go_vet_position_invalid")?
        .replace('\\', "/");
    Ok((display, line, column))
}
