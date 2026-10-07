//! TypeScript 方言原生检查的 tsconfig 输入绑定与必测反例（7.3 局部）。
//! 全部使用受控 shell 夹具模拟原生 Node/ESLint 协议，不冒充真实 ESLint 验收。
#![cfg(unix)]
use serde_json::Value;
use std::{
    fs,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    process::Command,
};

struct Project(PathBuf);
impl Drop for Project {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

/// 组装带本地 ESLint 候选的包目录；shim 按 mode 产出受控原生协议。
fn package(root: &Path, name: &str) -> PathBuf {
    let dir = root.join(name);
    fs::create_dir_all(dir.join("node_modules/eslint/bin")).unwrap();
    fs::write(
        dir.join("package.json"),
        r#"{"devDependencies":{"eslint":"10.11.0"}}"#,
    )
    .unwrap();
    fs::write(
        dir.join("node_modules/eslint/package.json"),
        r#"{"name":"eslint","version":"10.11.0","bin":{"eslint":"bin/eslint.js"}}"#,
    )
    .unwrap();
    fs::write(dir.join("node_modules/eslint/bin/eslint.js"), "fixture").unwrap();
    fs::write(dir.join("eslint.config.cjs"), "module.exports = [];\n").unwrap();
    fs::write(dir.join("tsconfig.json"), "{\"compilerOptions\":{}}\n").unwrap();
    dir
}

/// mode: clean=空报告 | finding=规则违规 | fatal=解析级致命 | bad=坏报告 | none=退出2不写报告
/// mutate_backend / mutate_all 在写报告后追加 tsconfig.json。
/// 仅为 app.ts 这类 TS 方言源码留下执行痕迹，用于断言不可信输入先于原生执行被拒绝。
const TS_MARKER: &str = "ts-native-executed";

fn node_shim(root: &Path, mode: &str, version: &str) {
    let body_mode = match mode {
        "mutate_backend" | "mutate_all" => "finding",
        other => other,
    };
    let body = match body_mode {
        "clean" => {
            "printf '[{\"filePath\":\"%s\",\"messages\":[],\"suppressedMessages\":[],\"errorCount\":0,\"warningCount\":0,\"fatalErrorCount\":0,\"fixableErrorCount\":0,\"fixableWarningCount\":0}]' \"$source\" > \"$report\"\nexit 0"
        }
        "finding" => {
            "printf '[{\"filePath\":\"%s\",\"messages\":[{\"ruleId\":\"no-debugger\",\"severity\":2,\"message\":\"fixture\",\"line\":1,\"column\":1}],\"suppressedMessages\":[],\"errorCount\":1,\"warningCount\":0,\"fatalErrorCount\":0,\"fixableErrorCount\":0,\"fixableWarningCount\":0}]' \"$source\" > \"$report\"\nexit 1"
        }
        "fatal" => {
            "printf '[{\"filePath\":\"%s\",\"messages\":[{\"ruleId\":null,\"fatal\":true,\"severity\":2,\"message\":\"Parsing error: dialect not covered\",\"line\":1,\"column\":1}],\"suppressedMessages\":[],\"errorCount\":1,\"warningCount\":0,\"fatalErrorCount\":1,\"fixableErrorCount\":0,\"fixableWarningCount\":0}]' \"$source\" > \"$report\"\nexit 1"
        }
        "bad" => "printf 'not json' > \"$report\"\nexit 0",
        "none" => "exit 2",
        _ => unreachable!(),
    }
    .to_owned();
    let mutation = match mode {
        "mutate_backend" => {
            "case \"$source\" in *backend*) printf '\\n//mutated' >> tsconfig.json;; esac\n"
        }
        "mutate_all" => "printf '\\n//mutated' >> tsconfig.json\n",
        _ => "",
    };
    let exit = match body_mode {
        "clean" | "bad" => "0",
        _ => "1",
    };
    let script = format!(
        "#!/bin/sh\nfor arg in \"$@\"; do if [ \"$arg\" = --version ]; then printf '{version}\\n'; exit 0; fi; done\nwhile [ \"$#\" -gt 0 ]; do if [ \"$1\" = --output-file ]; then shift; report=$1; fi; source=$1; shift; done\ncase \"$source\" in *.ts) printf used > '{}/{}';; esac\n{mutation}{body}\nexit {exit}\n",
        root.display(),
        TS_MARKER
    );
    let node = root.join("node");
    fs::write(&node, script).unwrap();
    fs::set_permissions(&node, fs::Permissions::from_mode(0o700)).unwrap();
}

fn project(label: &str, mode: &str) -> Project {
    project_versioned(label, mode, "v10.11.0")
}

fn project_versioned(label: &str, mode: &str, version: &str) -> Project {
    let root = std::env::temp_dir()
        .canonicalize()
        .unwrap()
        .join(format!("cg-eslint-tsconfig-{label}-{}", std::process::id()));
    fs::create_dir(&root).unwrap();
    let project = Project(root.clone());
    package(&root, "frontend");
    fs::write(root.join("frontend/app.ts"), "debugger;\n").unwrap();
    node_shim(&root, mode, version);
    project
}

fn check(root: &Path) -> Value {
    let out = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["check", "all"])
        .arg(root)
        .arg("--node-tool")
        .arg(root.join("node"))
        .args(["--format=json", "--timeout", "60s"])
        .output()
        .unwrap();
    assert_eq!(
        out.status.code(),
        Some(3),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    serde_json::from_slice(&out.stdout).unwrap()
}

fn app_row(report: &Value) -> &Value {
    report["native_results"]["node_lint"]["files"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["path"] == "frontend/app.ts")
        .unwrap()
}

#[test]
fn tsconfig_mutation_after_report_invalidates_native_observation() {
    // 对照轮：无篡改时同一夹具应得到局部一致观察。
    let clean = project("mutate-clean", "finding");
    let report = check(&clean.0);
    let row = app_row(&report);
    assert_eq!(row["feedback"]["local_coherent"], true, "{row}");
    assert_eq!(row["feedback"]["findings"][0]["rule_id"], "no-debugger");
    // 篡改轮：报告写完后 tsconfig.json 被改动，观察必须失效。
    let mutated = project("mutate-active", "mutate_all");
    let report = check(&mutated.0);
    let row = app_row(&report);
    assert_eq!(
        row["feedback"]["reason"],
        "eslint_input_changed",
        "{row}"
    );
    assert_eq!(row["feedback"]["local_coherent"], false);
    assert_eq!(row["source_sha256"], Value::Null);
    #[cfg(feature = "wasm-precheck")]
    {
        // 夹具自身的 eslint.config.cjs 是另一个 JS 方言源文件，可以独立一致；
        // 但被篡改 tsconfig 失效的 app.ts 不能被 WASM 初检替代或掩盖。
        assert!(report["syntax_candidates"]["observations"]
            .as_array()
            .unwrap()
            .iter()
            .any(|observation| observation["path"] == "frontend/app.ts"));
    }
    assert_eq!(report["delivery_decision"], "incomplete");
}

#[test]
fn symlinked_tsconfig_is_untrusted_and_blocks_native_execution() {
    let p = project("tsconfig-symlink", "finding");
    let real = p.0.join("frontend/tsconfig.real.json");
    fs::write(&real, "{\"compilerOptions\":{}}\n").unwrap();
    fs::remove_file(p.0.join("frontend/tsconfig.json")).unwrap();
    std::os::unix::fs::symlink(&real, p.0.join("frontend/tsconfig.json")).unwrap();
    let marker = p.0.join(TS_MARKER);
    let report = check(&p.0);
    let row = app_row(&report);
    assert_eq!(
        row["feedback"]["reason"],
        "eslint_tsconfig_untrusted",
        "{row}"
    );
    assert_eq!(row["feedback"]["local_coherent"], false);
    assert!(!marker.exists(), "不可信 tsconfig 不能先执行原生入口");
    assert_eq!(report["delivery_decision"], "incomplete");
}

#[test]
fn monorepo_tsconfig_mutation_only_invalidates_its_own_package() {
    let p = project("monorepo", "mutate_backend");
    let backend = package(&p.0, "backend");
    fs::write(backend.join("app.ts"), "debugger;\n").unwrap();
    let report = check(&p.0);
    let files = report["native_results"]["node_lint"]["files"]
        .as_array()
        .unwrap();
    let frontend = files
        .iter()
        .find(|row| row["path"] == "frontend/app.ts")
        .unwrap();
    assert_eq!(frontend["feedback"]["local_coherent"], true, "{frontend}");
    assert_eq!(
        frontend["feedback"]["findings"][0]["rule_id"],
        "no-debugger"
    );
    let backend_row = files
        .iter()
        .find(|row| row["path"] == "backend/app.ts")
        .unwrap();
    assert_eq!(
        backend_row["feedback"]["reason"],
        "eslint_input_changed",
        "{backend_row}"
    );
    assert_eq!(backend_row["feedback"]["local_coherent"], false);
}

#[test]
fn ts_parser_fatal_keeps_incomplete_and_names_parser_guidance() {
    let p = project("parser-fatal", "fatal");
    let report = check(&p.0);
    let row = app_row(&report);
    assert_eq!(
        row["feedback"]["reason"],
        "eslint_parser_or_configuration_diagnostic",
        "{row}"
    );
    assert_eq!(row["feedback"]["local_coherent"], false);
    assert_eq!(
        row["feedback"]["findings"]
            .as_array()
            .map(Vec::len)
            .unwrap_or(0),
        0,
        "致命解析诊断不能当成已归属规则发现"
    );
    let next = row["feedback"]["next_action"].as_str().unwrap();
    assert!(
        next.contains("解析级致命诊断") && next.contains("方言"),
        "缺少 parser/方言专属指引：{next}"
    );
    #[cfg(feature = "wasm-precheck")]
    assert_eq!(report["syntax_candidates"]["native_preferred_count"], 0);
    assert_eq!(report["delivery_decision"], "incomplete");
}

#[test]
fn invalid_config_exit_without_report_is_explicitly_incomplete() {
    let p = project("config-invalid", "none");
    let report = check(&p.0);
    let row = app_row(&report);
    assert_eq!(row["feedback"]["local_coherent"], false, "{row}");
    assert!(
        matches!(
            row["feedback"]["reason"].as_str(),
            Some("eslint_report_read_failed" | "eslint_execution_incomplete")
        ),
        "配置加载失败不能解释为干净：{row}"
    );
    assert_eq!(row["source_sha256"], Value::Null);
    #[cfg(feature = "wasm-precheck")]
    assert_eq!(report["syntax_candidates"]["native_preferred_count"], 0);
    assert_eq!(report["delivery_decision"], "incomplete");
}

#[test]
fn native_version_mismatch_is_reported_and_not_laundered() {
    let p = project_versioned("version-mismatch", "finding", "v10.11.1");
    let report = check(&p.0);
    let row = app_row(&report);
    assert_eq!(
        row["feedback"]["reason"],
        "eslint_version_mismatch",
        "{row}"
    );
    assert_eq!(row["feedback"]["local_coherent"], false);
    let next = row["feedback"]["next_action"].as_str().unwrap();
    assert!(
        next.contains("版本") && next.contains("不一致"),
        "版本不匹配需要专属指引：{next}"
    );
    #[cfg(feature = "wasm-precheck")]
    assert_eq!(report["syntax_candidates"]["native_preferred_count"], 0);
    assert_eq!(report["delivery_decision"], "incomplete");
}

#[test]
fn bad_report_is_rejected_without_clean_claim() {
    let p = project("bad-report", "bad");
    let report = check(&p.0);
    let row = app_row(&report);
    assert_eq!(row["feedback"]["reason"], "eslint_report_invalid", "{row}");
    assert_eq!(row["feedback"]["local_coherent"], false);
    assert_eq!(row["source_sha256"], Value::Null);
    assert_eq!(report["delivery_decision"], "incomplete");
}
