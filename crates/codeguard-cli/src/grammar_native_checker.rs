use crate::task_resolution_checker::TaskResolutionChecker;
use serde_json::Value;
use std::{path::Path, sync::atomic::AtomicBool, time::Instant};

/// 开发期语法差分工具选择；扩展回放不扩大可信任务关闭服务的授权范围。
#[derive(Clone, Copy)]
pub(crate) enum GrammarNativeChecker {
    /// 已有产品原生语法观察器。
    Existing(TaskResolutionChecker),
    /// Ruff 隔离语法观察，仅对固定 Python 3.12 语法作开发对照。
    Python,
    /// Node 的显式 ESM 语法观察，不推断项目模块类型。
    Javascript,
    /// Ruby 的显式隔离语法观察，不执行用户源码。
    Ruby,
    /// Go SDK 配对gofmt的整文件语法观察。
    Go,
    /// 显式固定edition2024的Rustfmt解析；不是Clippy lint或项目编译。
    Rust,
    /// 固定AppleClang21的C11独立源码观察。
    C,
    /// 固定AppleClang21的C++17独立源码观察。
    Cpp,
}

impl GrammarNativeChecker {
    /// 解析显式语言标识；未知语言不执行进程。
    pub(crate) fn for_language(language: &str) -> Option<Self> {
        match language {
            "zig" => Some(Self::Existing(TaskResolutionChecker::Zig)),
            "erlang" => Some(Self::Existing(TaskResolutionChecker::Erlang)),
            "swift" => Some(Self::Existing(TaskResolutionChecker::Swift)),
            "kotlin" => Some(Self::Existing(TaskResolutionChecker::Kotlin)),
            "python" => Some(Self::Python),
            "javascript" => Some(Self::Javascript),
            "ruby" => Some(Self::Ruby),
            "go" => Some(Self::Go),
            "rust" => Some(Self::Rust),
            "c" => Some(Self::C),
            "cpp" => Some(Self::Cpp),
            _ => None,
        }
    }
    /// 返回本次允许的原生版本，不从项目脚本推断。
    pub(crate) fn version(self) -> &'static str {
        match self {
            Self::Existing(checker) => checker.version(),
            Self::Python => "ruff 0.16.8",
            Self::Javascript => "v24.18.0",
            Self::Ruby => "ruby 2.6.10p210",
            Self::Go => "go1.23.4",
            Self::Rust => "rustfmt 1.9.0-stable",
            Self::C | Self::Cpp => "Apple clang version 21.0.0 (clang-2100.3.34.2)",
        }
    }
    /// 返回对应原生制品的有界字节预算；Node 独立预算不扩张其它工具权限。
    pub(crate) fn artifact_budget(self) -> u64 {
        match self {
            Self::Javascript => crate::javascript_syntax_probe::NODE_ARTIFACT_BUDGET,
            Self::C | Self::Cpp => 256 * 1024 * 1024,
            _ => 64 * 1024 * 1024,
        }
    }
    /// 冻结辅助制品入口；无辅助工具的观察器返回None，Go缺gofmt拒绝隐式替代。
    pub(crate) fn companion_identity(self, tool: &Path) -> Result<Option<String>, String> {
        match self {
            Self::Go => crate::go_syntax_probe::companion_identity(tool).map(Some),
            _ => Ok(None),
        }
    }
    /// 保留语法与语义的边界；Clang仅明确解析规则可分类为语法无效，警告不是语法错误。
    pub(crate) fn classify(self, native: &Value) -> Option<bool> {
        if matches!(self, Self::C | Self::Cpp) {
            if native["status"] == "completed" {
                return Some(true);
            }
            if native["status"] != "diagnostics_observed" {
                return None;
            }
            let rows = native["diagnostics"].as_array()?;
            let mut syntax_error = false;
            for row in rows {
                match row["level"].as_str()? {
                    "warning" | "note" => {}
                    "error" if row["rule_id"] == "clang.err_expected_expression" => {
                        syntax_error = true
                    }
                    // 类型、名称、扩展诊断及未审计解析规则不能被解释为grammar漏报。
                    _ => return None,
                }
            }
            Some(!syntax_error)
        } else {
            crate::grammar_native_differential::classify_native(native)
        }
    }
    /// 对冻结源码执行隔离语法观察；工具、源码与预算由差分入口绑定。
    pub(crate) fn observe(
        self,
        tool: &Path,
        source: &[u8],
        root: &Path,
        deadline: Instant,
        cancelled: &AtomicBool,
    ) -> Value {
        match self {
            Self::Existing(checker) => {
                checker.observe_with_cancellation(tool, source, root, deadline, cancelled)
            }
            Self::Python => crate::python_syntax_probe::observe(tool, source, deadline, cancelled),
            Self::Ruby => crate::ruby_syntax_probe::observe(tool, source, deadline, cancelled),
            Self::Go => crate::go_syntax_probe::observe(tool, source, deadline, cancelled),
            Self::Rust => crate::rustfmt_syntax_probe::observe(tool, source, deadline, cancelled),
            Self::C | Self::Cpp => crate::clang_syntax_probe::observe_with_cancellation(
                tool,
                if matches!(self, Self::C) { "c" } else { "cpp" },
                if matches!(self, Self::C) {
                    "c11"
                } else {
                    "c++17"
                },
                source,
                deadline,
                cancelled,
            ),
            Self::Javascript => {
                crate::javascript_syntax_probe::observe(tool, source, deadline, cancelled)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::GrammarNativeChecker;
    use std::{
        fs,
        os::unix::fs::PermissionsExt,
        sync::{
            Arc,
            atomic::{AtomicBool, Ordering},
        },
        thread,
        time::{Duration, Instant},
    };

    #[test]
    fn clang_syntax_labels_preserve_warning_and_semantic_boundaries() {
        for checker in [GrammarNativeChecker::C, GrammarNativeChecker::Cpp] {
            for (rows, expected) in [
                (
                    serde_json::json!([{"level":"warning","rule_id":"clang.warn_unused_variable"}]),
                    Some(true),
                ),
                (
                    serde_json::json!([{"level":"error","rule_id":"clang.err_expected_expression"}]),
                    Some(false),
                ),
                (
                    serde_json::json!([{"level":"error","rule_id":"clang.err_unknown_typename"}]),
                    None,
                ),
                (
                    serde_json::json!([{"level":"error","rule_id":"clang.err_expected_expression"},{"level":"error","rule_id":"clang.err_unknown_typename"}]),
                    None,
                ),
                (
                    serde_json::json!([{"rule_id":"clang.err_expected_expression"}]),
                    None,
                ),
            ] {
                assert_eq!(
                    checker.classify(
                        &serde_json::json!({"status":"diagnostics_observed","diagnostics":rows})
                    ),
                    expected
                );
            }
        }
    }

    #[test]
    fn every_native_observer_cancels_version_and_scan_in_flight() {
        for language in [
            "zig",
            "erlang",
            "swift",
            "kotlin",
            "python",
            "javascript",
            "ruby",
            "go",
            "rust",
            "c",
            "cpp",
        ] {
            for phase in ["version", "scan"] {
                let root = std::env::temp_dir().canonicalize().unwrap().join(format!(
                    "cg-native-cancel-{}-{language}-{phase}",
                    std::process::id()
                ));
                fs::create_dir(&root).unwrap();
                let tool = root.join("tool");
                let version = match language {
                    "zig" => "printf '0.16.0\\n'",
                    "erlang" => "printf 'OTP 28\\n'",
                    "swift" => "printf 'Apple Swift version 6.4 (fixture)\\n'",
                    "kotlin" => "printf 'info: kotlinc-jvm 2.4.10 (JRE fixture)\\n' >&2",
                    "python" => "printf 'ruff 0.16.8\\n'",
                    "ruby" => "printf 'ruby 2.6.10p210 (fixture) [fixture]\\n'",
                    "c" | "cpp" => "printf 'Apple clang version 21.0.0 (clang-2100.3.34.2)\\n'",
                    "rust" => "printf 'rustfmt 1.9.0-stable (fixture)\\n'",
                    "go" => {
                        "if [ \"$#\" = 1 ]; then printf 'go version go1.23.4 fixture/fixture\\n'; else printf '%s: go1.23.4\\n' \"$2\"; fi"
                    }
                    _ => "printf 'v24.18.0\\n'",
                };
                let block = "printf started > \"$0.started\"; exec /bin/sleep 30";
                let version_body = if phase == "version" { block } else { version };
                let script = format!(
                    "#!/bin/sh\ncase \"$*\" in version*|--version|-version|*system_info*) {version_body}; exit 0;; esac\n{block}\n"
                );
                fs::write(&tool, script).unwrap();
                fs::set_permissions(&tool, fs::Permissions::from_mode(0o700)).unwrap();
                if language == "go" {
                    let helper = root.join("gofmt");
                    fs::write(&helper, format!("#!/bin/sh\n{block}\n")).unwrap();
                    fs::set_permissions(&helper, fs::Permissions::from_mode(0o700)).unwrap();
                }
                let cancelled = Arc::new(AtomicBool::new(false));
                let trigger = Arc::clone(&cancelled);
                let marker = if language == "go" && phase == "scan" {
                    root.join("gofmt.started")
                } else {
                    tool.with_extension("started")
                };
                let watcher = thread::spawn(move || {
                    let deadline = Instant::now() + Duration::from_secs(4);
                    while !marker.exists() && Instant::now() < deadline {
                        thread::sleep(Duration::from_millis(5));
                    }
                    let started = marker.exists();
                    trigger.store(true, Ordering::Relaxed);
                    started
                });
                let started = Instant::now();
                let report = GrammarNativeChecker::for_language(language)
                    .unwrap()
                    .observe(
                        &tool,
                        b"x\n",
                        &root,
                        started + Duration::from_secs(10),
                        &cancelled,
                    );
                let actual_start = watcher.join().unwrap();
                let elapsed = started.elapsed();
                fs::remove_dir_all(root).unwrap();
                assert!(actual_start, "{language}/{phase}: 原生调用必须实际启动");
                assert_eq!(
                    report["status"], "incomplete",
                    "{language}/{phase}: {report}"
                );
                assert!(
                    elapsed < Duration::from_secs(5),
                    "{language}/{phase}: 不能等待工具自行完成，耗时{elapsed:?}"
                );
                assert!(
                    report["diagnostics"].as_array().unwrap().is_empty(),
                    "取消不能伪造语法发现"
                );
            }
        }
    }
    #[test]
    fn every_observer_stops_before_scan_when_version_changes_entry() {
        for language in [
            "zig",
            "erlang",
            "swift",
            "kotlin",
            "python",
            "javascript",
            "ruby",
            "go",
            "rust",
            "c",
            "cpp",
        ] {
            for mode in ["alias", "replace"] {
                let root = std::env::temp_dir().canonicalize().unwrap().join(format!(
                    "cg-native-entry-{}-{language}-{mode}",
                    std::process::id()
                ));
                fs::create_dir(&root).unwrap();
                let tool = root.join("tool");
                let other = tool.with_extension("other");
                let alias = tool.with_extension("alias");
                fs::write(
                    &other,
                    "#!/bin/sh\nprintf reached >> \"$0.called\"\nexit 0\n",
                )
                .unwrap();
                fs::set_permissions(&other, fs::Permissions::from_mode(0o700)).unwrap();
                let version = match language {
                    "zig" => "printf '0.16.0\\n'",
                    "erlang" => "printf 'OTP 28\\n'",
                    "swift" => "printf 'Apple Swift version 6.4 (fixture)\\n'",
                    "kotlin" => "printf 'info: kotlinc-jvm 2.4.10 (JRE fixture)\\n' >&2",
                    "python" => "printf 'ruff 0.16.8\\n'",
                    "ruby" => "printf 'ruby 2.6.10p210 (fixture) [fixture]\\n'",
                    "c" | "cpp" => "printf 'Apple clang version 21.0.0 (clang-2100.3.34.2)\\n'",
                    "rust" => "printf 'rustfmt 1.9.0-stable (fixture)\\n'",
                    "go" => {
                        "if [ \"$#\" = 1 ]; then printf 'go version go1.23.4 fixture/fixture\\n'; else printf '%s: go1.23.4\\n' \"$2\"; fi"
                    }
                    _ => "printf 'v24.18.0\\n'",
                };
                let action = if mode == "alias" {
                    "/bin/ln -sf \"$0.other\" \"$0.alias\""
                } else {
                    "/bin/mv \"$0.other\" \"$0\""
                };
                fs::write(&tool,format!("#!/bin/sh\ncase \"$*\" in version*|--version|-version|*system_info*) {action}; {version}; exit 0;; esac\nprintf reached >> \"$0.called\"\nexit 0\n")).unwrap();
                fs::set_permissions(&tool, fs::Permissions::from_mode(0o700)).unwrap();
                if language == "go" {
                    let helper = root.join("gofmt");
                    fs::write(
                        &helper,
                        "#!/bin/sh\nprintf reached > \"$0.called\"\n/bin/cat\n",
                    )
                    .unwrap();
                    fs::set_permissions(&helper, fs::Permissions::from_mode(0o700)).unwrap();
                }
                std::os::unix::fs::symlink(&tool, &alias).unwrap();
                let report = GrammarNativeChecker::for_language(language)
                    .unwrap()
                    .observe(
                        &alias,
                        b"x\n",
                        &root,
                        Instant::now() + Duration::from_secs(5),
                        &AtomicBool::new(false),
                    );
                let mut redirected_marker = other.as_os_str().to_owned();
                redirected_marker.push(".called");
                let called = root.join("gofmt.called").exists()
                    || tool.with_extension("called").exists()
                    || std::path::PathBuf::from(redirected_marker).exists();
                fs::remove_dir_all(root).unwrap();
                assert!(
                    !called,
                    "{language}/{mode}: 版本后的入口变化不得继续执行源码调用"
                );
                assert_eq!(
                    report["status"], "incomplete",
                    "{language}/{mode}: {report}"
                );
                assert!(report["diagnostics"].as_array().unwrap().is_empty());
            }
        }
    }
    #[test]
    fn kotlin_version_cannot_change_frozen_source_before_scan() {
        let root = std::env::temp_dir()
            .canonicalize()
            .unwrap()
            .join(format!("cg-kotlin-version-input-{}", std::process::id()));
        fs::create_dir(&root).unwrap();
        let tool = root.join("tool");
        fs::write(&tool,"#!/bin/sh\nif [ \"$1\" = -version ]; then printf changed > Sample.kt; printf 'info: kotlinc-jvm 2.4.10 (JRE fixture)\\n' >&2; exit 0; fi\nprintf reached > \"$0.called\"\nexit 0\n").unwrap();
        fs::set_permissions(&tool, fs::Permissions::from_mode(0o700)).unwrap();
        let report = GrammarNativeChecker::for_language("kotlin")
            .unwrap()
            .observe(
                &tool,
                b"fun f() = 1\n",
                &root,
                Instant::now() + Duration::from_secs(5),
                &AtomicBool::new(false),
            );
        let called = root.join("gofmt.called").exists() || tool.with_extension("called").exists();
        fs::remove_dir_all(root).unwrap();
        assert!(!called, "冻结源码已改变时不得启动编译动作");
        assert_eq!(report["status"], "incomplete");
        assert_eq!(
            report["reason"],
            "kotlin_input_or_launcher_changed_during_check"
        );
        assert!(report["diagnostics"].as_array().unwrap().is_empty());
    }

    #[test]
    fn zig_version_stderr_is_not_accepted_as_confirmed_version() {
        let root = std::env::temp_dir()
            .canonicalize()
            .unwrap()
            .join(format!("cg-zig-version-stderr-{}", std::process::id()));
        fs::create_dir(&root).unwrap();
        let tool = root.join("tool");
        fs::write(&tool,"#!/bin/sh\nif [ \"$1\" = version ]; then printf '0.16.0\\n'; printf unexpected >&2; exit 0; fi\nprintf reached > \"$0.called\"\nexit 0\n").unwrap();
        fs::set_permissions(&tool, fs::Permissions::from_mode(0o700)).unwrap();
        let report = GrammarNativeChecker::for_language("zig").unwrap().observe(
            &tool,
            b"const x = 1;\n",
            &root,
            Instant::now() + Duration::from_secs(5),
            &AtomicBool::new(false),
        );
        let called = root.join("gofmt.called").exists() || tool.with_extension("called").exists();
        fs::remove_dir_all(root).unwrap();
        assert!(!called, "版本stderr非空不得启动AST调用");
        assert_eq!(report["status"], "incomplete");
        assert_eq!(report["reason"], "zig_version_unverified_or_unsupported");
    }
}
