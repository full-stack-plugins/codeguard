//! 固定 Go SDK 的整文件语法开发对照；不解析导入、不执行源码或代替 go vet。
use codeguard_runtime::{ProcessSpec, Termination, read_bounded_regular_file, run_process};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    ffi::OsString,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    sync::atomic::AtomicBool,
    time::Instant,
};

const ARTIFACT_BUDGET: u64 = 64 * 1024 * 1024;

/// 冻结显式SDK入口同目录gofmt的规范入口及字节身份。
/// 参数为请求Go绝对入口；返回有界联合摘要，不从PATH寻找辅助工具。
pub(crate) fn companion_identity(tool: &Path) -> Result<String, String> {
    let (_, canonical, sha) =
        companion(tool).map_err(|_| "native_grammar_companion_unavailable".to_owned())?;
    let mut hash = Sha256::new();
    hash.update(canonical.as_os_str().as_encoded_bytes());
    hash.update([0]);
    hash.update(sha.as_bytes());
    Ok(format!("{:x}", hash.finalize()))
}

fn companion(tool: &Path) -> Result<(PathBuf, PathBuf, String), ()> {
    if !tool.is_absolute() {
        return Err(());
    }
    let requested = tool.parent().ok_or(())?.join("gofmt");
    let canonical = requested.canonicalize().map_err(|_| ())?;
    let metadata = std::fs::metadata(&canonical).map_err(|_| ())?;
    if !metadata.is_file() || metadata.permissions().mode() & 0o111 == 0 {
        return Err(());
    }
    let bytes = read_bounded_regular_file(&canonical, ARTIFACT_BUDGET).map_err(|_| ())?;
    Ok((requested, canonical, digest(&bytes)))
}

/// 以冻结stdin调用同SDK gofmt的整文件模式；参数含共同预算和取消。
/// 返回绑定两个入口的开发观察，失败不提供语法通过或正式lint权威。
pub(crate) fn observe(
    tool: &Path,
    source: &[u8],
    deadline: Instant,
    cancelled: &AtomicBool,
) -> Value {
    let mut report = json!({"status":"incomplete","reason":"go_syntax_input_invalid","version":null,"tool_sha256":null,"gofmt_sha256":null,"companion_binding_sha256":null,"input_type":"whole_file","diagnostics":[]});
    if !tool.is_absolute() || source.len() > 1024 * 1024 || std::str::from_utf8(source).is_err() {
        return report;
    }
    // Go扫描器允许line指令重映射位置；未建立逻辑到原始位置映射时不猜原始锚点。
    if source
        .windows(7)
        .any(|window| window == b"//line " || window == b"/*line ")
    {
        report["reason"] = json!("go_syntax_logical_positions_unresolved");
        return report;
    }
    report["reason"] = json!("go_syntax_tool_unavailable");
    let Ok(executable) = tool.canonicalize() else {
        return report;
    };
    let Ok(bytes) = read_bounded_regular_file(&executable, ARTIFACT_BUDGET) else {
        return report;
    };
    let sha = digest(&bytes);
    report["tool_sha256"] = json!(sha);
    let Ok((fmt_request, gofmt, fmt_sha)) = companion(tool) else {
        return report;
    };
    let Ok(binding) = companion_identity(tool) else {
        return report;
    };
    report["gofmt_sha256"] = json!(fmt_sha);
    report["companion_binding_sha256"] = json!(binding);
    let current = || {
        tool.canonicalize().ok().as_deref() == Some(executable.as_path())
            && read_bounded_regular_file(&executable, ARTIFACT_BUDGET)
                .is_ok_and(|bytes| digest(&bytes) == sha)
            && fmt_request.canonicalize().ok().as_deref() == Some(gofmt.as_path())
            && companion_identity(tool).is_ok_and(|value| value == binding)
    };
    let env = BTreeMap::from([
        (OsString::from("GOTOOLCHAIN"), OsString::from("local")),
        (OsString::from("GOENV"), OsString::from("off")),
        (OsString::from("GOWORK"), OsString::from("off")),
        (OsString::from("GOPROXY"), OsString::from("off")),
        (OsString::from("GOSUMDB"), OsString::from("off")),
    ]);
    let invoke = |executable: &Path, args: Vec<OsString>, stdin| {
        run_process(
            &ProcessSpec {
                executable: executable.to_path_buf(),
                args,
                cwd: PathBuf::from("/"),
                env: env.clone(),
                stdin,
                deadline,
                output_limit_bytes: 64 * 1024,
            },
            cancelled,
        )
    };
    let version = invoke(&executable, vec!["version".into()], None);
    if !current() {
        report["reason"] = json!("go_syntax_tool_changed");
        return report;
    }
    if version.termination != Termination::Exited(0)
        || !version.stderr.is_empty()
        || !verified_version(&version.stdout)
    {
        report["reason"] = json!("go_syntax_version_unverified");
        return report;
    }
    let fmt_version = invoke(
        &executable,
        vec!["version".into(), gofmt.as_os_str().to_owned()],
        None,
    );
    if !current() {
        report["reason"] = json!("go_syntax_tool_changed");
        return report;
    }
    let mut expected = gofmt.as_os_str().as_encoded_bytes().to_vec();
    expected.extend_from_slice(b": go1.23.4\n");
    if fmt_version.termination != Termination::Exited(0)
        || !fmt_version.stderr.is_empty()
        || fmt_version.stdout != expected
    {
        report["reason"] = json!("go_syntax_version_unverified");
        return report;
    }
    report["version"] = json!("go1.23.4");
    let outcome = invoke(
        &gofmt,
        vec!["-e".into(), "/dev/stdin".into()],
        Some(source.to_vec()),
    );
    if !current() {
        report["reason"] = json!("go_syntax_tool_changed");
        return report;
    }
    if !matches!(outcome.termination, Termination::Exited(0 | 2)) {
        report["reason"] = json!("go_syntax_execution_incomplete");
        return report;
    }
    let success = outcome.termination == Termination::Exited(0);
    let diagnostics = if success
        && !outcome.stdout.is_empty()
        && outcome.stderr.is_empty()
        && std::str::from_utf8(&outcome.stdout).is_ok()
    {
        Some(Vec::new())
    } else if !success && outcome.stdout.is_empty() {
        diagnostics(&outcome.stderr, source)
    } else {
        None
    };
    let Some(diagnostics) = diagnostics else {
        report["reason"] = json!("go_syntax_report_invalid");
        return report;
    };
    report["diagnostics"] = json!(diagnostics);
    report["status"] = json!(if success {
        "completed"
    } else {
        "diagnostics_observed"
    });
    report["reason"] = json!(if success {
        "go_native_syntax_no_diagnostics"
    } else {
        "go_native_syntax_diagnostics"
    });
    report
}
fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn verified_version(stdout: &[u8]) -> bool {
    std::str::from_utf8(stdout)
        .ok()
        .and_then(|text| text.strip_suffix('\n'))
        .and_then(|text| text.strip_prefix("go version go1.23.4 "))
        .is_some_and(|platform| {
            !platform.chars().any(char::is_whitespace)
                && platform
                    .split_once('/')
                    .is_some_and(|(os, arch)| !os.is_empty() && !arch.is_empty())
        })
}
fn diagnostics(stderr: &[u8], source: &[u8]) -> Option<Vec<Value>> {
    let text = std::str::from_utf8(stderr).ok()?;
    let source = std::str::from_utf8(source).ok()?;
    // go/token把终止换行保留在最后一行；末尾EOF仍锚定该行的换行之后。
    let mut lines: Vec<&str> = source.split_inclusive('\n').collect();
    if lines.is_empty() {
        lines.push("");
    }
    let mut positions = BTreeSet::new();
    for line in text.lines() {
        let line = line.strip_prefix("/dev/stdin:")?;
        let (row, rest) = line.split_once(':')?;
        let (column, message) = rest.split_once(": ")?;
        let row = row.parse::<usize>().ok()?;
        let column = column.parse::<usize>().ok()?;
        let original = lines.get(row.checked_sub(1)?)?;
        let byte = column.checked_sub(1)?;
        if message.is_empty()
            || byte > original.len()
            || (byte == original.len() && row != lines.len())
            || !original.is_char_boundary(byte)
        {
            return None;
        }
        positions.insert((row, column));
        if positions.len() > 32 {
            return None;
        }
    }
    if positions.is_empty() {
        return None;
    }
    Some(positions.into_iter().map(|(line,column)|json!({"line":line,"column":column,"column_unit":"utf8_byte","rule_id":"go.syntax"})).collect())
}

#[cfg(test)]
mod tests {
    use super::{diagnostics, observe, verified_version};
    use std::{
        fs,
        os::unix::fs::PermissionsExt,
        path::PathBuf,
        sync::{
            Arc,
            atomic::{AtomicBool, Ordering},
        },
        thread,
        time::{Duration, Instant},
    };
    fn root(name: &str) -> PathBuf {
        let root = std::env::temp_dir()
            .canonicalize()
            .unwrap()
            .join(format!("cg-go-{name}-{}", std::process::id()));
        fs::create_dir(&root).unwrap();
        root
    }
    fn executable(path: &std::path::Path, script: &str) {
        fs::write(path, script).unwrap();
        fs::set_permissions(path, fs::Permissions::from_mode(0o700)).unwrap();
    }
    const GO_VERSION: &str = "if [ \"$#\" = 1 ]; then printf 'go version go1.23.4 fixture/fixture\\n'; else printf '%s: go1.23.4\\n' \"$2\"; fi";
    #[test]
    fn version_and_stdin_byte_positions_must_be_complete_and_consistent() {
        assert!(verified_version(b"go version go1.23.4 darwin/arm64\n"));
        assert!(!verified_version(b"go version go1.23.5 darwin/arm64\n"));
        assert!(!verified_version(
            b"go version go1.23.4 darwin/arm64\nnoise\n"
        ));
        // Go的终止换行不增加最后一行；EOF列包含该换行的原始字节。
        assert_eq!(
            diagnostics(
                b"/dev/stdin:2:9: expected operand, found 'EOF'\n",
                b"package p\nvar x =\n"
            )
            .unwrap()[0]["column"],
            9
        );
        let source = "package p\nvar 名称 =\n".as_bytes();
        assert_eq!(
            diagnostics(b"/dev/stdin:2:13: expected operand, found 'EOF'\n", source).unwrap()[0]["column"],
            13
        );
        for stderr in [
            b"other.go:1:1: expected token\n".as_slice(),
            b"/dev/stdin:0:1: expected token\n",
            b"/dev/stdin:2:6: expected token\n",
            b"/dev/stdin:1:1: expected token\nfatal runtime\n",
            b"/dev/stdin:1:1: \n",
            b"\xff",
        ] {
            assert!(diagnostics(stderr, source).is_none(), "{stderr:?}");
        }
        let source = "x\n".repeat(33);
        let stderr = (1..=33)
            .map(|line| format!("/dev/stdin:{line}:1: expected token\n"))
            .collect::<String>();
        assert!(diagnostics(stderr.as_bytes(), source.as_bytes()).is_none());
    }
    #[test]
    fn both_sdk_versions_and_format_exit_must_match_the_contract() {
        let root = root("outputs");
        let go = root.join("go");
        let fmt = root.join("gofmt");
        executable(&go, &format!("#!/bin/sh\n{GO_VERSION}\n"));
        for (body, status) in [
            ("/bin/cat", "completed"),
            (
                "/bin/cat >/dev/null; printf '/dev/stdin:2:1: expected declaration\\n' >&2; exit 2",
                "diagnostics_observed",
            ),
            ("/bin/cat >/dev/null; exit 0", "incomplete"),
            ("/bin/cat; exit 2", "incomplete"),
            ("/bin/cat; printf warning >&2; exit 0", "incomplete"),
            (
                "/bin/cat >/dev/null; printf 'fatal runtime\\n' >&2; exit 2",
                "incomplete",
            ),
            ("/bin/cat >/dev/null; exit 1", "incomplete"),
        ] {
            executable(&fmt, &format!("#!/bin/sh\n{body}\n"));
            let report = observe(
                &go,
                b"package p\nvar x =\n",
                Instant::now() + Duration::from_secs(5),
                &AtomicBool::new(false),
            );
            assert_eq!(report["status"], status, "{body}: {report}");
        }
        executable(
            &go,
            "#!/bin/sh\nif [ \"$#\" = 1 ]; then printf 'go version go1.23.4 fixture/fixture\\n'; else printf '%s: go1.23.5\\n' \"$2\"; fi\n",
        );
        executable(
            &fmt,
            "#!/bin/sh\nprintf reached > \"$0.called\"\n/bin/cat\n",
        );
        let report = observe(
            &go,
            b"package p\n",
            Instant::now() + Duration::from_secs(5),
            &AtomicBool::new(false),
        );
        assert_eq!(report["reason"], "go_syntax_version_unverified");
        assert!(!fmt.with_extension("called").exists());
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn changing_either_companion_entry_or_bytes_stops_source_execution() {
        for phase in ["version", "helper_version"] {
            for mode in ["replace", "alias"] {
                let root = root(&format!("entry-{phase}-{mode}"));
                let go = root.join("go");
                let fmt = root.join("gofmt");
                let real = root.join("fmt_real");
                let other = root.join("fmt_other");
                executable(
                    &real,
                    "#!/bin/sh\nprintf reached > \"$0.called\"\n/bin/cat\n",
                );
                executable(
                    &other,
                    "#!/bin/sh\n# different frozen artifact\nprintf reached > \"$0.called\"\n/bin/cat\n",
                );
                std::os::unix::fs::symlink(&real, &fmt).unwrap();
                let action = if mode == "alias" {
                    "/bin/ln -sf \"$base/fmt_other\" \"$base/gofmt\""
                } else {
                    "/bin/mv \"$base/fmt_other\" \"$base/fmt_real\""
                };
                let condition = if phase == "version" {
                    "[ \"$#\" = 1 ]"
                } else {
                    "[ \"$#\" = 2 ]"
                };
                executable(
                    &go,
                    &format!(
                        "#!/bin/sh\nbase=${{0%/*}}\nif {condition}; then {action}; fi\n{GO_VERSION}\n"
                    ),
                );
                let report = observe(
                    &go,
                    b"package p\n",
                    Instant::now() + Duration::from_secs(5),
                    &AtomicBool::new(false),
                );
                assert_eq!(report["reason"], "go_syntax_tool_changed");
                assert_eq!(report["diagnostics"], serde_json::json!([]));
                assert!(!real.with_extension("called").exists());
                assert!(!other.with_extension("called").exists());
                fs::remove_dir_all(root).unwrap();
            }
        }
    }
    #[test]
    fn every_go_native_phase_observes_in_flight_request_cancellation() {
        for phase in ["version", "helper_version", "scan"] {
            let root = root(&format!("cancel-{phase}"));
            let go = root.join("go");
            let fmt = root.join("gofmt");
            let block = "printf started > \"$0.started\"; exec /bin/sleep 30";
            let condition = match phase {
                "version" => "[ \"$#\" = 1 ]",
                "helper_version" => "[ \"$#\" = 2 ]",
                _ => "false",
            };
            executable(
                &go,
                &format!("#!/bin/sh\nif {condition}; then {block}; fi\n{GO_VERSION}\n"),
            );
            executable(&fmt, &format!("#!/bin/sh\n{block}\n"));
            let marker = if phase == "scan" {
                fmt.with_extension("started")
            } else {
                go.with_extension("started")
            };
            let cancelled = Arc::new(AtomicBool::new(false));
            let trigger = Arc::clone(&cancelled);
            let watcher = thread::spawn(move || {
                let deadline = Instant::now() + Duration::from_secs(4);
                while !marker.exists() && Instant::now() < deadline {
                    thread::sleep(Duration::from_millis(5));
                }
                let actual = marker.exists();
                trigger.store(true, Ordering::Relaxed);
                actual
            });
            let started = Instant::now();
            let report = observe(
                &go,
                b"package p\n",
                started + Duration::from_secs(10),
                &cancelled,
            );
            assert!(watcher.join().unwrap(), "{phase}: native phase must start");
            assert!(
                started.elapsed() < Duration::from_secs(5),
                "{phase}: cancellation did not terminate the active process"
            );
            assert_eq!(report["status"], "incomplete");
            assert_eq!(report["diagnostics"], serde_json::json!([]));
            fs::remove_dir_all(root).unwrap();
        }
    }
    #[test]
    fn logical_line_directives_cannot_forge_original_positions() {
        let report = observe(
            std::path::Path::new("/unavailable/go"),
            b"package p\n//line /dev/stdin:1\nvar x =\n",
            Instant::now(),
            &AtomicBool::new(false),
        );
        assert_eq!(report["reason"], "go_syntax_logical_positions_unresolved");
        assert_eq!(report["diagnostics"], serde_json::json!([]));
    }
}
