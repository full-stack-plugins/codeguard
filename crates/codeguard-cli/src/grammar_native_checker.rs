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
        }
    }
    /// 返回对应原生制品的有界字节预算；Node 独立预算不扩张其它工具权限。
    pub(crate) fn artifact_budget(self) -> u64 {
        match self {
            Self::Javascript => crate::javascript_syntax_probe::NODE_ARTIFACT_BUDGET,
            _ => 64 * 1024 * 1024,
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
    fn every_native_observer_cancels_version_and_scan_in_flight() {
        for language in [
            "zig",
            "erlang",
            "swift",
            "kotlin",
            "python",
            "javascript",
            "ruby",
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
                    _ => "printf 'v24.18.0\\n'",
                };
                let block = "printf started > \"$0.started\"; exec /bin/sleep 30";
                let version_body = if phase == "version" { block } else { version };
                let script = format!(
                    "#!/bin/sh\ncase \"$*\" in version|--version|-version|*system_info*) {version_body}; exit 0;; esac\n{block}\n"
                );
                fs::write(&tool, script).unwrap();
                fs::set_permissions(&tool, fs::Permissions::from_mode(0o700)).unwrap();
                let cancelled = Arc::new(AtomicBool::new(false));
                let trigger = Arc::clone(&cancelled);
                let marker = tool.with_extension("started");
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
                    _ => "printf 'v24.18.0\\n'",
                };
                let action = if mode == "alias" {
                    "/bin/ln -sf \"$0.other\" \"$0.alias\""
                } else {
                    "/bin/mv \"$0.other\" \"$0\""
                };
                fs::write(&tool,format!("#!/bin/sh\ncase \"$*\" in version|--version|-version|*system_info*) {action}; {version}; exit 0;; esac\nprintf reached >> \"$0.called\"\nexit 0\n")).unwrap();
                fs::set_permissions(&tool, fs::Permissions::from_mode(0o700)).unwrap();
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
                let called = tool.with_extension("called").exists()
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
        let called = tool.with_extension("called").exists();
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
        let called = tool.with_extension("called").exists();
        fs::remove_dir_all(root).unwrap();
        assert!(!called, "版本stderr非空不得启动AST调用");
        assert_eq!(report["status"], "incomplete");
        assert_eq!(report["reason"], "zig_version_unverified_or_unsupported");
    }
}
