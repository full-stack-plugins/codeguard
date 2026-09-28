//! 原生 Git 完整提交祖先观察；工具与仓库的可信来源仍由宿主负责。

use std::collections::BTreeMap;
use std::ffi::OsString;
use std::path::Path;
use std::sync::atomic::AtomicBool;
use std::time::Instant;

use crate::{ProcessOutcome, ProcessSpec, Termination, run_process};

/// 在共同截止时间内只读观察两个完整提交 ID 的祖先关系。
///
/// 返回 true 包括同一提交，false 仅表示原生完整历史未匹配；浅历史、
/// 非提交对象、缺对象及执行失败返回错误。调用方仍须核验 Git 身份及仓库隔离。
pub fn observe_git_ancestry(
    root: &Path,
    git_tool: &Path,
    ancestor: &str,
    descendant: &str,
    deadline: Instant,
    cancelled: &AtomicBool,
) -> Result<bool, &'static str> {
    if !root.is_absolute()
        || !git_tool.is_absolute()
        || !root.is_dir()
        || !valid_oid(ancestor)
        || !valid_oid(descendant)
        || ancestor.len() != descendant.len()
    {
        return Err("git_ancestry_invalid_input");
    }
    let run = |args: &[&str]| {
        let mut env = BTreeMap::new();
        for (key, value) in [
            ("GIT_CONFIG_NOSYSTEM", "1"),
            ("GIT_CONFIG_GLOBAL", "/dev/null"),
            ("GIT_NO_REPLACE_OBJECTS", "1"),
            ("GIT_GRAFT_FILE", "/dev/null"),
            ("GIT_NO_LAZY_FETCH", "1"),
            ("GIT_TERMINAL_PROMPT", "0"),
            ("LC_ALL", "C"),
        ] {
            env.insert(OsString::from(key), OsString::from(value));
        }
        let mut argv: Vec<OsString> = [
            "--no-lazy-fetch",
            "--no-replace-objects",
            "--no-optional-locks",
            "-c",
            "core.commitGraph=false",
            "-c",
            "advice.graftFileDeprecated=false",
        ]
        .into_iter()
        .map(OsString::from)
        .collect();
        argv.extend(args.iter().map(OsString::from));
        run_process(
            &ProcessSpec {
                executable: git_tool.into(),
                args: argv,
                cwd: root.into(),
                env,
                stdin: None,
                deadline,
                output_limit_bytes: 64 * 1024,
            },
            cancelled,
        )
    };
    let shallow = run(&["rev-parse", "--is-shallow-repository"]);
    successful_output(&shallow)?;
    match shallow.stdout.as_slice() {
        b"false\n" => {}
        b"true\n" => return Err("git_ancestry_shallow_history"),
        _ => return Err("git_ancestry_invalid_report"),
    }
    for oid in [ancestor, descendant] {
        let object = run(&["cat-file", "-t", oid]);
        successful_output(&object)?;
        if object.stdout != b"commit\n" {
            return Err("git_ancestry_non_commit_object");
        }
    }
    let relation = run(&["merge-base", "--is-ancestor", ancestor, descendant]);
    if !relation.stdout.is_empty() || !relation.stderr.is_empty() {
        return Err("git_ancestry_invalid_report");
    }
    match relation.termination {
        Termination::Exited(0) => Ok(true),
        Termination::Exited(1) => Ok(false),
        _ => Err("git_ancestry_execution_incomplete"),
    }
}

fn successful_output(outcome: &ProcessOutcome) -> Result<(), &'static str> {
    if outcome.termination != Termination::Exited(0) {
        return Err("git_ancestry_execution_incomplete");
    }
    if !outcome.stderr.is_empty() {
        return Err("git_ancestry_invalid_report");
    }
    Ok(())
}

fn valid_oid(value: &str) -> bool {
    matches!(value.len(), 40 | 64)
        && value.bytes().any(|b| b != b'0')
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}
