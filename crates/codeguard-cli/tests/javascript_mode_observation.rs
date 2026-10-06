#![cfg(unix)]
use codeguard_cli::javascript_mode_observation::observe_javascript_mode;
use serde_json::Value;
use std::{
    fs,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};
static NEXT: AtomicU64 = AtomicU64::new(0);
struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        let root = std::env::temp_dir().canonicalize().unwrap().join(format!(
            "cg-js-mode-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&root).unwrap();
        Self(root)
    }
    fn source(&self, path: &str) {
        let full = self.0.join(path);
        fs::create_dir_all(full.parent().unwrap()).unwrap();
        fs::write(full, b"return 1;\n").unwrap();
    }
    fn observe(&self, path: &str) -> Value {
        let observation = observe_javascript_mode(&self.0, Path::new(path));
        let value = serde_json::to_value(observation).unwrap();
        if let Some(dir) = std::env::var_os("CODEGUARD_JS_MODE_REPORT_DIR") {
            fs::create_dir_all(&dir).unwrap();
            fs::write(
                PathBuf::from(dir).join(format!(
                    "{}-{}.json",
                    std::process::id(),
                    NEXT.fetch_add(1, Ordering::Relaxed)
                )),
                serde_json::to_vec(&value).unwrap(),
            )
            .unwrap();
        }
        value
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
#[test]
fn explicit_extensions_override_package_type_without_reading_package() {
    let root = Fixture::new();
    fs::write(root.0.join("package.json"), b"broken").unwrap();
    for (path, mode, basis) in [
        ("src/file.mjs", "module", "extension_mjs"),
        ("src/file.cjs", "commonjs", "extension_cjs"),
    ] {
        root.source(path);
        let row = root.observe(path);
        assert_eq!(row["mode"], mode);
        assert_eq!(row["basis"], basis);
        assert!(row["package_sha256"].is_null());
        assert!(row["searched_directories"].as_array().unwrap().is_empty());
        assert!(row["source_sha256"].as_str().is_some());
        assert_eq!(row["native_execution"], "not_run");
    }
}
#[test]
fn nearest_explicit_package_and_changes_bind_source_evidence() {
    let root = Fixture::new();
    root.source("app/src/file.js");
    fs::write(root.0.join("package.json"), b"{\"type\":\"module\"}").unwrap();
    let first = root.observe("app/src/file.js");
    assert_eq!(first["mode"], "module");
    assert_eq!(first["package_path"], "package.json");
    fs::write(root.0.join("app/package.json"), b"{\"type\":\"commonjs\"}").unwrap();
    let nearer = root.observe("app/src/file.js");
    assert_eq!(nearer["mode"], "commonjs");
    assert_eq!(nearer["package_path"], "app/package.json");
    assert_ne!(first, nearer);
    fs::write(root.0.join("app/package.json"), b"{}").unwrap();
    let unknown = root.observe("app/src/file.js");
    assert_eq!(unknown["mode"], "unknown");
    assert_eq!(unknown["package_path"], "app/package.json");
    assert_ne!(unknown["package_sha256"], nearer["package_sha256"]);
    fs::remove_file(root.0.join("app/package.json")).unwrap();
    assert_eq!(first, root.observe("app/src/file.js"));
    fs::write(root.0.join("app/src/file.js"), b"const x=1;\n").unwrap();
    assert_ne!(
        first["source_sha256"],
        root.observe("app/src/file.js")["source_sha256"]
    );
}
#[test]
fn absent_invalid_and_ambiguous_types_do_not_guess_or_inherit() {
    let root = Fixture::new();
    root.source("src/file.js");
    let absent = root.observe("src/file.js");
    assert_eq!(absent["mode"], "unknown");
    assert_eq!(absent["reason"], "package_type_not_observed");
    assert_eq!(
        absent["searched_directories"],
        serde_json::json!(["src", "."])
    );
    fs::write(root.0.join("package.json"), b"{\"type\":\"module\"}").unwrap();
    for bytes in [
        b"{".as_slice(),
        b"[]",
        b"{\"type\":4}",
        b"{\"type\":\"other\"}",
        b"{\"type\":\"module\",\"type\":\"commonjs\"}",
        b"{}",
    ] {
        fs::write(root.0.join("src/package.json"), bytes).unwrap();
        let row = root.observe("src/file.js");
        assert_eq!(row["mode"], "unknown");
        assert_eq!(row["package_path"], "src/package.json");
        assert!(row["package_sha256"].as_str().is_some());
    }
}
#[test]
fn root_boundaries_links_and_budgets_are_unknown_without_tool_execution() {
    let outer = Fixture::new();
    let root = Fixture::new();
    root.source("src/file.js");
    outer.source("checked/src/file.js");
    fs::write(outer.0.join("package.json"), b"{\"type\":\"module\"}").unwrap();
    let bounded = observe_javascript_mode(&outer.0.join("checked"), Path::new("src/file.js"));
    assert_eq!(bounded.mode, "unknown");
    assert!(bounded.package_path.is_none());
    assert_eq!(bounded.searched_directories, ["src", "."]);
    std::os::unix::fs::symlink(
        outer.0.join("package.json"),
        root.0.join("src/package.json"),
    )
    .unwrap();
    let linked = root.observe("src/file.js");
    assert_eq!(linked["mode"], "unknown");
    assert_eq!(linked["reason"], "package_path_untrusted");
    fs::remove_file(root.0.join("src/package.json")).unwrap();
    fs::write(root.0.join("src/package.json"), vec![b' '; 256 * 1024 + 1]).unwrap();
    assert_eq!(root.observe("src/file.js")["mode"], "unknown");
    std::os::unix::fs::symlink(root.0.join("src/file.js"), root.0.join("linked.mjs")).unwrap();
    assert_eq!(root.observe("linked.mjs")["mode"], "unknown");
    assert_eq!(root.observe("../outside.js")["mode"], "unknown");
    assert_eq!(root.observe("/outside.js")["mode"], "unknown");
    root.source("file.jsx");
    assert_eq!(root.observe("file.jsx")["mode"], "unknown");
    let deep = (0..65).map(|_| "d").collect::<Vec<_>>().join("/") + "/file.js";
    root.source(&deep);
    let depth = root.observe(&deep);
    assert_eq!(depth["mode"], "unknown");
    assert_eq!(depth["reason"], "package_search_budget_exhausted");
    assert_eq!(depth["searched_directories"].as_array().unwrap().len(), 64);
}

#[test]
#[ignore = "需显式已有Node24.18.0，对照真实包/后缀模式，不批准语言资格"]
fn actual_node_file_check_matches_explicit_local_mode_boundaries() {
    use codeguard_runtime::{ProcessSpec, Termination, run_process};
    use std::{
        collections::BTreeMap,
        ffi::OsString,
        sync::atomic::AtomicBool,
        time::{Duration, Instant},
    };
    let tool = PathBuf::from(std::env::var_os("CODEGUARD_NODE_BIN").expect("显式指定已有Node"));
    assert!(tool.is_absolute());
    let executable = tool.canonicalize().unwrap();
    let before = fs::read(&executable).unwrap();
    let root = Fixture::new();
    let deadline = Instant::now() + Duration::from_secs(30);
    let cancelled = AtomicBool::new(false);
    let invoke = |args: Vec<OsString>| {
        run_process(
            &ProcessSpec {
                executable: executable.clone(),
                args,
                cwd: root.0.clone(),
                env: BTreeMap::new(),
                stdin: None,
                deadline,
                output_limit_bytes: 65536,
            },
            &cancelled,
        )
    };
    let version = invoke(vec!["--version".into()]);
    assert_eq!(version.termination, Termination::Exited(0));
    assert_eq!(version.stdout, b"v24.18.0\n");
    assert!(version.stderr.is_empty());
    fs::write(root.0.join("package.json"), b"{\"type\":\"module\"}").unwrap();
    root.source("app/src/nested.js");
    root.source("unknown/file.js");
    fs::write(root.0.join("app/package.json"), b"{\"type\":\"commonjs\"}").unwrap();
    fs::write(root.0.join("unknown/package.json"), b"{}").unwrap();
    for (path, mode, invalid) in [
        ("file.mjs", "module", true),
        ("file.cjs", "commonjs", false),
        ("file.js", "module", true),
        ("app/src/nested.js", "commonjs", false),
        ("unknown/file.js", "unknown", false),
    ] {
        root.source(path);
        let observed = observe_javascript_mode(&root.0, Path::new(path));
        assert_eq!(observed.mode, mode);
        let outcome = invoke(vec!["--check".into(), root.0.join(path).into_os_string()]);
        assert_eq!(
            outcome.termination,
            Termination::Exited(i32::from(invalid)),
            "{path}"
        );
        assert!(outcome.stdout.is_empty());
        if invalid {
            assert!(String::from_utf8_lossy(&outcome.stderr).contains("SyntaxError"));
        } else {
            assert!(outcome.stderr.is_empty());
        }
        assert_eq!(observed, observe_javascript_mode(&root.0, Path::new(path)));
        assert_eq!(fs::read(root.0.join(path)).unwrap(), b"return 1;\n");
    }
    assert_eq!(tool.canonicalize().unwrap(), executable);
    assert_eq!(fs::read(executable).unwrap(), before);
}
