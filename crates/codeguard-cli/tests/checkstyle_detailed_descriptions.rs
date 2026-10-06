#![cfg(unix)]

use serde_json::{Value, json};
use std::{
    fs,
    path::PathBuf,
    process::Command,
    sync::atomic::{AtomicU64, Ordering},
};

static NEXT: AtomicU64 = AtomicU64::new(0);
const DTD: &str = "<!DOCTYPE module PUBLIC \"-//Checkstyle//DTD Checkstyle Configuration 1.3//EN\" \"https://checkstyle.org/dtds/configuration_1_3.dtd\">";
struct Fixture {
    root: PathBuf,
    java: PathBuf,
    jar: PathBuf,
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}
impl Fixture {
    fn new(actual: bool) -> Self {
        let root = std::env::temp_dir().canonicalize().unwrap().join(format!(
            "cg-checkstyle-detail-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&root).unwrap();
        fs::write(root.join("Foo.java"), "/** */ public class Foo {}\n").unwrap();
        let (java, jar) = if actual {
            (
                PathBuf::from(std::env::var_os("CODEGUARD_JAVA_BIN").unwrap())
                    .canonicalize()
                    .unwrap(),
                PathBuf::from(std::env::var_os("CODEGUARD_CHECKSTYLE_JAR").unwrap())
                    .canonicalize()
                    .unwrap(),
            )
        } else {
            // 编译受控XML运输夹具以走真实进程/快照路径；该程序不是Java或Checkstyle，不能证明原插件精度。
            let source = root.join("transport.rs");
            fs::write(&source,r#"use std::{env,fs};
fn main() {
 let args:Vec<_> = env::args().collect(); let source=args.last().unwrap();
 let xml=if fs::read_to_string(source).unwrap().contains("Documented") { format!("<checkstyle version=\"10.21.4\"><file name=\"{}\"/></checkstyle>",source) }
 else { format!("<checkstyle version=\"10.21.4\"><file name=\"{}\"><error line=\"1\" column=\"1\" severity=\"warning\" message=\"controlled description diagnostic\" source=\"detailDocs\"/></file></checkstyle>",source) };
 fs::write(&args[args.iter().position(|v|v=="-o").unwrap()+1],xml).unwrap();
}"#).unwrap();
            let java = root.join("transport");
            let compiler = std::env::var_os("RUSTC").unwrap_or_else(|| "rustc".into());
            let built = Command::new(compiler)
                .args(["--edition=2021", "--crate-name=checkstyle_transport"])
                .arg(&source)
                .arg("-o")
                .arg(&java)
                .output()
                .unwrap();
            assert!(built.status.success(), "{built:?}");
            let jar = root.join("fixture.jar");
            fs::write(&jar, b"controlled transport identity, not a native jar").unwrap();
            (java, jar)
        };
        let out = Command::new(env!("CARGO_BIN_EXE_codeguard"))
            .args(["init", root.to_str().unwrap(), "--apply"])
            .output()
            .unwrap();
        assert_eq!(out.status.code(), Some(3));
        Self { root, java, jar }
    }
    fn configure(&self, name: &str, props: &str) {
        fs::write(self.root.join("checks.xml"),format!("{DTD}<module name=\"Checker\"><module name=\"TreeWalker\"><module name=\"{name}\"><property name=\"id\" value=\"detailDocs\"/>{props}</module></module></module>")).unwrap();
    }
    fn run(&self, args: &[&str], tools: bool) -> Value {
        let mut cmd = Command::new(env!("CARGO_BIN_EXE_codeguard"));
        cmd.args(args);
        if args[0] == "lint" {
            cmd.arg(self.root.join("Foo.java"))
                .arg("--checker=checkstyle")
                .arg("--workspace")
                .arg(&self.root);
        } else {
            cmd.arg(&self.root);
        }
        if tools {
            cmd.arg("--java-tool")
                .arg(&self.java)
                .arg("--checkstyle-jar")
                .arg(&self.jar)
                .arg("--config")
                .arg(self.root.join("checks.xml"));
        }
        let out = cmd.arg("--format=json").output().unwrap();
        assert!(matches!(out.status.code(), Some(0 | 3)), "{out:?}");
        serde_json::from_slice(&out.stdout).unwrap()
    }
}

#[test]
fn controlled_description_classes_reach_original_task_recheck_without_protocol_downgrade() {
    let mut reports = Vec::new();
    for (module, properties) in [
        (
            "JavadocStyle",
            "<property name=\"checkEmptyJavadoc\" value=\"true\"/>",
        ),
        (
            "NonEmptyAtclauseDescription",
            "<property name=\"javadocTokens\" value=\"PARAM_LITERAL,RETURN_LITERAL,THROWS_LITERAL\"/>",
        ),
        ("SummaryJavadoc", "<property name=\"period\" value=\"。\"/>"),
    ] {
        let f = Fixture::new(false);
        f.configure(module, properties);
        let first = f.run(&["lint", "java"], true);
        assert_eq!(first["schema_version"], "0.5.0");
        assert_eq!(first["local_status"], "observed", "{first}");
        assert_eq!(first["workbench"]["new_findings"], 1);
        let id = first["workbench"]["next"]["repair_brief"]["task_id"]
            .as_str()
            .unwrap();
        let next = f.run(&["next"], false);
        assert_eq!(next["schema_version"], "0.25.0");
        assert_eq!(
            next["repair_brief"]["checkstyle_guidance"]["checker_class"],
            format!("com.puppycrawl.tools.checkstyle.checks.javadoc.{module}Check")
        );
        let present = f.run(&["task", "verify", id], true);
        assert_eq!(present["schema_version"], "0.33.0");
        assert_eq!(present["observation"], "still_present", "{present}");
        assert_eq!(present["event_persisted"], true);
        let reportpath = fs::read_dir(f.root.join(".codeguard/reports"))
            .unwrap()
            .map(|v| v.unwrap().path())
            .find(|p| {
                p.file_name()
                    .unwrap()
                    .to_str()
                    .unwrap()
                    .starts_with("checkstyle-")
                    && !p
                        .file_name()
                        .unwrap()
                        .to_str()
                        .unwrap()
                        .starts_with("checkstyle-task-")
            })
            .unwrap();
        let saved: Value = serde_json::from_slice(&fs::read(reportpath).unwrap()).unwrap();
        assert_eq!(saved["schema_version"], "0.2.0");
        let mut downgraded = saved.clone();
        downgraded["run_id"] = json!("checkstyle-downgraded");
        downgraded["schema_version"] = json!("0.1.0");
        fs::write(
            f.root.join(".codeguard/reports/checkstyle-downgraded.json"),
            serde_json::to_vec(&downgraded).unwrap(),
        )
        .unwrap();
        let sync = f.run(&["work", "sync"], false);
        assert_eq!(sync["failed_reports"], 1);
        fs::remove_file(f.root.join(".codeguard/reports/checkstyle-downgraded.json")).unwrap();
        let aggregate = f.run(&["check", "java"], false);
        assert_eq!(aggregate["delivery_decision"], "not_evaluated");
        let comments = aggregate["category_candidates"]
            .as_array()
            .unwrap()
            .iter()
            .find(|v| v["language"] == "java" && v["category"] == "comments")
            .unwrap();
        assert_eq!(comments["reason"], "javadoc_checker_not_configured");
        fs::write(
            f.root.join("Foo.java"),
            "/** Documented. */ public class Foo {}\n",
        )
        .unwrap();
        let absent = f.run(&["task", "verify", id], true);
        assert_eq!(
            absent["observation"], "candidate_absent_unverified_policy",
            "{absent}"
        );
        let fact: Value = serde_json::from_slice(
            &fs::read(
                f.root
                    .join(format!(".codeguard/findings/{id}/finding.json")),
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(fact["state"], "open");
        reports.push(json!({"evidence_kind":"controlled_checkstyle_xml_transport_not_native_qualification","module":module,"feedback":first,"workbench":saved,"next":next,"present":present,"absent":absent,"aggregate":aggregate}));
    }
    if let Some(path) = std::env::var_os("CODEGUARD_TEST_CHECKSTYLE_DETAILED_FIXTURE_EVIDENCE") {
        fs::write(path, serde_json::to_vec_pretty(&reports).unwrap()).unwrap();
    }
}

#[test]
#[ignore = "需要显式已有Java及Checkstyle10.21.4自包含JAR，不安装下载；原描述规则尚待实测"]
fn actual_checkstyle_detailed_modules_preserve_source_tasks_and_repaired_absence() {
    for (module, properties, source) in [
        (
            "JavadocStyle",
            "<property name=\"checkEmptyJavadoc\" value=\"true\"/><property name=\"checkFirstSentence\" value=\"false\"/><property name=\"checkHtml\" value=\"false\"/>",
            "/** */ public class Foo {}\n",
        ),
        (
            "NonEmptyAtclauseDescription",
            "",
            "/** Foo. */ public class Foo {\n/** Computes.\n * @param value\n * @return\n * @throws IllegalArgumentException\n */\npublic int run(int value) throws IllegalArgumentException { return value; }\n}\n",
        ),
        ("SummaryJavadoc", "", "/** */ public class Foo {}\n"),
    ] {
        let f = Fixture::new(true);
        f.configure(module, properties);
        fs::write(f.root.join("Foo.java"), source).unwrap();
        let report = f.run(&["lint", "java"], true);
        assert_eq!(report["local_status"], "observed", "{report}");
        let findings = report["findings"].as_array().unwrap();
        assert!(!findings.is_empty(), "{report}");
        assert!(findings.iter().all(|v| v["checker_class"]
            == format!("com.puppycrawl.tools.checkstyle.checks.javadoc.{module}Check")));
        let ids: Vec<_> = fs::read_dir(f.root.join(".codeguard/findings"))
            .unwrap()
            .map(|v| v.unwrap().file_name().into_string().unwrap())
            .collect();
        assert_eq!(ids.len(), findings.len());
        for id in &ids {
            let present = f.run(&["task", "verify", id], true);
            assert_eq!(present["observation"], "still_present", "{present}");
        }
        fs::write(
            f.root.join("Foo.java"),
            "/** Documented. */ public class Foo {}\n",
        )
        .unwrap();
        for id in &ids {
            let absent = f.run(&["task", "verify", id], true);
            assert_eq!(
                absent["observation"], "candidate_absent_unverified_policy",
                "{absent}"
            );
        }
    }
}

#[test]
fn preparation_recovery_preserves_detailed_scan_and_allows_wrapped_source_task_recheck() {
    let f = Fixture::new(false);
    f.configure(
        "JavadocStyle",
        "<property name=\"checkEmptyJavadoc\" value=\"true\"/>",
    );
    let blocked = f.run(&["lint", "java"], false);
    let id = blocked["workbench"]["next"]["repair_brief"]["task_id"]
        .as_str()
        .unwrap();
    let recovered = f.run(&["task", "verify", id], true);
    assert_eq!(recovered["schema_version"], "0.34.0");
    assert_eq!(
        recovered["observation"], "environment_restored_unverified_policy",
        "{recovered}"
    );
    assert_eq!(recovered["event_persisted"], true);
    let next = f.run(&["next"], false);
    assert_eq!(next["repair_brief"]["checker_id"], "java.checkstyle");
    let source = next["repair_brief"]["task_id"].as_str().unwrap();
    let present = f.run(&["task", "verify", source], true);
    assert_eq!(present["observation"], "still_present", "{present}");
    assert_eq!(present["event_persisted"], true);
    if let Some(path) = std::env::var_os("CODEGUARD_TEST_CHECKSTYLE_DETAILED_RECOVERY_EVIDENCE") {
        fs::write(path,serde_json::to_vec_pretty(&json!({"evidence_kind":"controlled_checkstyle_xml_transport_recovery_not_native_qualification","blocked":blocked,"recovered":recovered,"next":next,"present":present})).unwrap()).unwrap();
    }
}

#[test]
fn removing_the_original_description_class_requires_review_and_keeps_a_valid_recheck_event() {
    let f = Fixture::new(false);
    f.configure(
        "JavadocStyle",
        "<property name=\"checkEmptyJavadoc\" value=\"true\"/>",
    );
    let first = f.run(&["lint", "java"], true);
    let id = first["workbench"]["next"]["repair_brief"]["task_id"]
        .as_str()
        .unwrap();
    f.configure("MissingJavadocType", "");
    let report = f.run(&["task", "verify", id], true);
    assert_eq!(
        report["observation"], "rule_coverage_requires_review",
        "{report}"
    );
    assert_eq!(report["event_persisted"], true, "{report}");
    assert_eq!(report["native_scan"]["schema_version"], "0.2.0");
    assert_eq!(report["native_scan"]["scan"]["schema_version"], "0.2.0");
    let fact: Value = serde_json::from_slice(
        &fs::read(
            f.root
                .join(format!(".codeguard/findings/{id}/finding.json")),
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(fact["state"], "open");
    if let Some(path) = std::env::var_os("CODEGUARD_TEST_CHECKSTYLE_RULE_REVIEW_EVIDENCE") {
        fs::write(path, serde_json::to_vec_pretty(&report).unwrap()).unwrap();
    }
}
