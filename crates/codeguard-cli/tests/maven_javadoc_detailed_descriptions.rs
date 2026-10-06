#![cfg(unix)]

use codeguard_cli::tool_identity::hash_bundle_tree;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    fs,
    os::unix::fs::PermissionsExt,
    path::PathBuf,
    process::Command,
    sync::atomic::{AtomicU64, Ordering},
};

static NEXT: AtomicU64 = AtomicU64::new(0);
struct Fixture {
    root: PathBuf,
    tool: PathBuf,
    home: PathBuf,
    repo: PathBuf,
}
impl Fixture {
    fn new() -> Self {
        let root = std::env::temp_dir().canonicalize().unwrap().join(format!(
            "cg-maven-detailed-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(root.join("src/main/java")).unwrap();
        fs::write(
            root.join("src/main/java/Sample.java"),
            "/** */ public class Sample {}\n",
        )
        .unwrap();
        fs::write(
            root.join("src/main/java/Helper.java"),
            "/** Helper. */ public class Helper { /** Creates helper. */ public Helper() {} }\n",
        )
        .unwrap();
        fs::write(root.join("pom.xml"),"<project><modelVersion>4.0.0</modelVersion><groupId>demo</groupId><artifactId>demo</artifactId><version>1</version><build><plugins><plugin><artifactId>maven-javadoc-plugin</artifactId><version>3.12.0</version><configuration><doclint>missing</doclint></configuration></plugin></plugins></build></project>\n").unwrap();
        let home = root.join("jdk");
        fs::create_dir_all(home.join("bin")).unwrap();
        fs::write(home.join("bin/java"), "fixture runtime").unwrap();
        fs::write(home.join("release"), "JAVA_VERSION=\"21.0.12\"\n").unwrap();
        let repo = root.join("repo");
        fs::create_dir(&repo).unwrap();
        let tool = root.join("mvn");
        Self {
            root,
            tool,
            home,
            repo,
        }
    }
    fn initialize(&self) {
        let out = Command::new(env!("CARGO_BIN_EXE_codeguard"))
            .arg("init")
            .arg(&self.root)
            .arg("--apply")
            .output()
            .unwrap();
        assert_eq!(out.status.code(), Some(3));
    }
    fn controlled_tool(&self, message: &str, exit: i32) {
        let footer = if exit == 0 {
            "[INFO] BUILD SUCCESS\\n"
        } else {
            "[INFO] BUILD FAILURE\\n[ERROR] Failed to execute goal org.apache.maven.plugins:maven-javadoc-plugin:3.12.0:javadoc (default-cli) on project demo: Project contains Javadoc Warnings -> [Help 1]\\n"
        };
        fs::write(&self.tool,format!("#!/bin/sh\nIFS= read -r source_line < src/main/java/Sample.java\nmkdir -p target/reports/apidocs\nprintf '<html></html>' > target/reports/apidocs/index.html\nif /usr/bin/grep -Fq Documented src/main/java/Sample.java; then printf '[INFO] --- javadoc:3.12.0:javadoc (default-cli) @ demo ---\\n[INFO] BUILD SUCCESS\\n'; exit 0; fi\nprintf '[INFO] --- javadoc:3.12.0:javadoc (default-cli) @ demo ---\\n[WARNING] Javadoc Warnings\\n[WARNING] %s:1: warning: {message}\\n[WARNING] %s\\n[WARNING] ^\\n[WARNING] 1 warning\\n{footer}' \"$PWD/src/main/java/Sample.java\" \"$source_line\"\nexit {exit}\n")).unwrap();
        fs::set_permissions(&self.tool, fs::Permissions::from_mode(0o700)).unwrap();
    }
    fn run(&self, command: &[&str], tools: bool) -> Value {
        let mut cmd = Command::new(env!("CARGO_BIN_EXE_codeguard"));
        cmd.args(command).arg(&self.root);
        if tools {
            cmd.arg("--maven-tool")
                .arg(&self.tool)
                .arg("--java-home")
                .arg(&self.home)
                .arg("--maven-repo")
                .arg(&self.repo)
                .arg("--repo-sha256")
                .arg(hash_bundle_tree(&self.repo).unwrap());
        }
        let out = cmd.arg("--format=json").output().unwrap();
        assert!(matches!(out.status.code(), Some(0 | 3)), "{out:?}");
        serde_json::from_slice(&out.stdout).unwrap()
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

#[test]
fn controlled_success_and_warning_failure_preserve_each_detailed_rule_through_original_task() {
    let mut reports = Vec::new();
    for (message, rule) in [
        ("empty comment", "JavadocEmptyComment"),
        ("no main description", "JavadocMissingMainDescription"),
        ("no description for @param", "JavadocEmptyParamDescription"),
        (
            "no description for @return",
            "JavadocEmptyReturnDescription",
        ),
        (
            "no description for @throws",
            "JavadocEmptyThrowsDescription",
        ),
    ] {
        for exit in [0, 1] {
            let f = Fixture::new();
            f.controlled_tool(message, exit);
            f.initialize();
            let feedback = f.run(&["comments", "java"], true);
            assert_eq!(feedback["schema_version"], "0.10.0");
            assert_eq!(
                feedback["workbench"]["status"], "synced_partial",
                "{feedback}"
            );
            assert_eq!(feedback["workbench"]["new_findings"], 1);
            let native =
                &feedback["native_observation"]["maven_multifile_probes"][0]["observation"];
            assert_eq!(native["findings"][0]["rule_id"], rule);
            assert_eq!(native["source_count"], 2);
            let brief = &feedback["workbench"]["next"]["repair_brief"];
            let id = brief["task_id"].as_str().unwrap();
            assert_eq!(brief["schema_version"], "0.6.0");
            let repeat = f.run(&["comments", "java"], true);
            assert_eq!(repeat["workbench"]["new_findings"], 0);
            assert_eq!(repeat["workbench"]["next"]["repair_brief"]["task_id"], id);
            let present = f.run(&["task", "verify", id], true);
            assert_eq!(present["schema_version"], "0.32.0");
            assert_eq!(present["observation"], "still_present", "{present}");
            assert_eq!(present["event_persisted"], true);
            let next = f.run(&["next"], false);
            assert_eq!(
                next["repair_brief"]["verification_observation"],
                "still_present"
            );
            assert!(
                next["repair_brief"]["step"]
                    .as_str()
                    .unwrap()
                    .contains("详细说明")
            );
            let aggregate = f.run(&["check", "java"], true);
            assert_eq!(aggregate["schema_version"], "0.69.0");
            let path = fs::read_dir(f.root.join(".codeguard/reports"))
                .unwrap()
                .map(|p| p.unwrap().path())
                .find(|p| {
                    p.file_name()
                        .unwrap()
                        .to_str()
                        .unwrap()
                        .starts_with("javadoc-maven-")
                        && !p
                            .file_name()
                            .unwrap()
                            .to_str()
                            .unwrap()
                            .starts_with("javadoc-maven-task-")
                })
                .unwrap();
            let first: Value = serde_json::from_slice(&fs::read(path).unwrap()).unwrap();
            fs::write(f.root.join("src/main/java/Sample.java"),"/** Documented. */ public class Sample { /** Creates sample. */ public Sample() {} }\n").unwrap();
            let absent = f.run(&["task", "verify", id], true);
            assert_eq!(
                absent["observation"], "candidate_absent_unverified_policy",
                "{absent}"
            );
            assert_eq!(absent["event_persisted"], true);
            let fact: Value = serde_json::from_slice(
                &fs::read(
                    f.root
                        .join(format!(".codeguard/findings/{id}/finding.json")),
                )
                .unwrap(),
            )
            .unwrap();
            assert_eq!(fact["state"], "open");
            reports.push(json!({"evidence_kind":"controlled_maven_transport_fixture","message":message,"native_exit":exit,"feedback":feedback,"workbench":first,"next":next,"aggregate":aggregate,"present":present,"absent":absent}));
        }
    }
    if let Some(path) = std::env::var_os("CODEGUARD_TEST_MAVEN_DETAILED_FIXTURE_EVIDENCE") {
        fs::write(path, serde_json::to_vec_pretty(&reports).unwrap()).unwrap();
    }
}

#[test]
fn first_import_rejects_detailed_rule_protocol_downgrade() {
    let f = Fixture::new();
    f.controlled_tool("empty comment", 0);
    f.initialize();
    let report = f.run(&["comments", "java"], true);
    assert_eq!(report["workbench"]["new_findings"], 1);
    let path = fs::read_dir(f.root.join(".codeguard/reports"))
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path();
    let first: Value = serde_json::from_slice(&fs::read(path).unwrap()).unwrap();
    for (name, outer, project, native) in [
        ("outer", "0.1.0", "0.5.0", "0.2.0"),
        ("project", "0.2.0", "0.3.0", "0.2.0"),
        ("native", "0.2.0", "0.5.0", "0.1.0"),
        ("all", "0.1.0", "0.3.0", "0.1.0"),
    ] {
        let mut forged = first.clone();
        let run = format!("javadoc-maven-downgrade-{name}");
        forged["run_id"] = json!(run);
        forged["schema_version"] = json!(outer);
        forged["native"]["schema_version"] = json!(project);
        forged["native"]["maven_multifile_probes"][0]["observation"]["schema_version"] =
            json!(native);
        fs::write(
            f.root.join(format!(".codeguard/reports/{run}.json")),
            serde_json::to_vec(&forged).unwrap(),
        )
        .unwrap();
    }
    let sync = f.run(&["work", "sync"], false);
    assert_eq!(sync["failed_reports"], 4, "{sync}");
    assert_eq!(
        fs::read_dir(f.root.join(".codeguard/tasks"))
            .unwrap()
            .count(),
        1
    );
}

#[test]
#[ignore = "需要显式已有Maven/JDK21；仅实测空离线库缺插件的环境路径，不安装插件"]
fn actual_maven_missing_offline_plugin_is_only_an_environment_task() {
    let mut f = Fixture::new();
    f.tool = PathBuf::from(std::env::var_os("CODEGUARD_MAVEN_BIN").unwrap())
        .canonicalize()
        .unwrap();
    f.home = PathBuf::from(std::env::var_os("CODEGUARD_TEST_JAVA_HOME").unwrap());
    f.initialize();
    let binary = env!("CARGO_BIN_EXE_codeguard");
    let sha = format!("{:x}", Sha256::digest(fs::read(binary).unwrap()));
    let feedback = f.run(&["comments", "java"], true);
    let native = &feedback["native_observation"]["maven_multifile_probes"][0]["observation"];
    assert_eq!(native["native_status"], "incomplete");
    assert_eq!(
        native["reason"], "maven_javadoc_offline_plugin_unavailable",
        "{feedback}"
    );
    assert_eq!(feedback["workbench"]["new_findings"], 0);
    assert_eq!(feedback["workbench"]["new_blockers"], 1);
    let id = feedback["workbench"]["next"]["repair_brief"]["task_id"]
        .as_str()
        .unwrap();
    let verify = f.run(&["task", "verify", id], true);
    assert_eq!(verify["observation"], "incomplete");
    assert_eq!(verify["event_persisted"], true);
    assert_eq!(
        verify["native_scan"]["scan"]["native"]["maven_multifile_probes"][0]["observation"]["reason"],
        "maven_javadoc_offline_plugin_unavailable"
    );
    assert_eq!(
        format!("{:x}", Sha256::digest(fs::read(binary).unwrap())),
        sha
    );
    if let Some(path) = std::env::var_os("CODEGUARD_TEST_MAVEN_OFFLINE_EVIDENCE") {
        fs::write(path,serde_json::to_vec_pretty(&json!({"evidence_kind":"actual_existing_maven_offline_missing_plugin","codeguard_binary_sha256":sha,"feedback":feedback,"verify":verify})).unwrap()).unwrap();
    }
}

#[test]
fn a_new_anchor_requires_review_and_its_wrapped_first_evidence_can_be_rechecked() {
    let f = Fixture::new();
    f.controlled_tool("empty comment", 0);
    f.initialize();
    let first = f.run(&["comments", "java"], true);
    let old = first["workbench"]["next"]["repair_brief"]["task_id"]
        .as_str()
        .unwrap();
    fs::write(
        f.root.join("src/main/java/Sample.java"),
        "/** */ public class Sample { public int value; }\n",
    )
    .unwrap();
    let verify = f.run(&["task", "verify", old], true);
    assert_eq!(
        verify["observation"], "rule_coverage_requires_review",
        "{verify}"
    );
    assert_eq!(verify["event_persisted"], true);
    let new = verify["native_scan"]["scan"]["findings"][0]["finding_id"]
        .as_str()
        .unwrap();
    assert_ne!(old, new);
    let second = f.run(&["task", "verify", new], true);
    assert_eq!(second["observation"], "still_present", "{second}");
    assert_eq!(second["event_persisted"], true);
}

#[test]
#[ignore = "需要显式已有Maven/JDK21和Javadoc3.12.0完整离线依赖缓存；不安装下载"]
fn actual_cached_maven_detailed_comments_preserve_original_tasks() {
    let tool = PathBuf::from(std::env::var_os("CODEGUARD_MAVEN_BIN").expect("existing Maven"))
        .canonicalize()
        .unwrap();
    let home = PathBuf::from(std::env::var_os("CODEGUARD_TEST_JAVA_HOME").expect("existing JDK21"));
    let repo = PathBuf::from(
        std::env::var_os("CODEGUARD_JAVADOC_MAVEN_REPO")
            .expect("existing complete offline repository"),
    );
    let documented = "/** 提供数值计算示例。 */\npublic class Sample {\n /** 创建计算器。 */ public Sample() {}\n /** 输出结果的初始值。 */ public int value;\n /** 返回输入数值。\n  * @param input 待返回的输入数值\n  * @return {@code input} 的原值\n  * @throws IllegalArgumentException 输入为负数时抛出\n  */\n public int run(int input) throws IllegalArgumentException { if (input < 0) { throw new IllegalArgumentException(); } return input; }\n /** {@inheritDoc} */\n @Override public String toString() { return \"sample\"; }\n}\n";
    for fail_on_warnings in [false, true] {
        for (source, rules) in [
            (
                "/** */\npublic class Sample {\n /** */ public Sample() {}\n /** */ public int value;\n /** */ public void run() {}\n}\n",
                vec!["JavadocEmptyComment"; 4],
            ),
            (
                "/** Sample API. */\npublic class Sample { /** Creates sample. */ public Sample() {}\n/** Computes value.\n * @param value\n * @return\n * @throws IllegalArgumentException\n */\npublic int run(int value) throws IllegalArgumentException { return value; } }\n",
                vec![
                    "JavadocEmptyParamDescription",
                    "JavadocEmptyReturnDescription",
                    "JavadocEmptyThrowsDescription",
                ],
            ),
            (
                "/** Sample API. */\npublic class Sample { /** Creates sample. */ public Sample() {}\n/** @param value the input\n * @return the value\n */\npublic int run(int value) { return value; } }\n",
                vec!["JavadocMissingMainDescription"],
            ),
            (documented, vec![]),
        ] {
            let mut f = Fixture::new();
            f.tool = tool.clone();
            f.home = home.clone();
            f.repo = repo.clone();
            fs::write(f.root.join("src/main/java/Sample.java"), source).unwrap();
            let pom = fs::read_to_string(f.root.join("pom.xml")).unwrap().replace(
                "<doclint>missing</doclint>",
                &format!(
                    "<doclint>missing</doclint><failOnWarnings>{fail_on_warnings}</failOnWarnings>"
                ),
            );
            fs::write(f.root.join("pom.xml"), pom).unwrap();
            f.initialize();
            let report = f.run(&["comments", "java"], true);
            let findings = report["native_observation"]["maven_multifile_probes"][0]["observation"]
                ["findings"]
                .as_array()
                .unwrap();
            let native = &report["native_observation"]["maven_multifile_probes"][0]["observation"];
            assert_eq!(
                native["native_status"],
                if rules.is_empty() {
                    "clean_log_unverified"
                } else {
                    "findings_observed_untrusted"
                },
                "{report}"
            );
            let mut actual: Vec<_> = findings
                .iter()
                .map(|v| v["rule_id"].as_str().unwrap())
                .collect();
            let mut expected = rules;
            actual.sort();
            expected.sort();
            assert_eq!(actual, expected, "{report}");
            let ids: Vec<_> = fs::read_dir(f.root.join(".codeguard/findings"))
                .unwrap()
                .map(|v| v.unwrap().file_name().into_string().unwrap())
                .filter(|id| {
                    let fact: Value = serde_json::from_slice(
                        &fs::read(
                            f.root
                                .join(format!(".codeguard/findings/{id}/finding.json")),
                        )
                        .unwrap(),
                    )
                    .unwrap();
                    fact["kind"] == "finding"
                })
                .collect();
            assert_eq!(ids.len(), expected.len(), "{report}");
            for id in &ids {
                let present = f.run(&["task", "verify", id], true);
                assert_eq!(present["observation"], "still_present", "{present}");
            }
            fs::write(f.root.join("src/main/java/Sample.java"), documented).unwrap();
            for id in &ids {
                let absent = f.run(&["task", "verify", id], true);
                assert_eq!(
                    absent["observation"], "candidate_absent_unverified_policy",
                    "{absent}"
                );
            }
        }
    }
}

#[test]
fn uninitialized_maven_feedback_preserves_the_detailed_native_contract_without_tasks() {
    let f = Fixture::new();
    f.controlled_tool("empty comment", 0);
    let report = f.run(&["comments", "java"], true);
    assert_eq!(report["schema_version"], "0.9.0");
    assert_eq!(
        report["native_observation"]["maven_multifile_probes"][0]["observation"]["findings"][0]["rule_id"],
        "JavadocEmptyComment"
    );
    assert!(!f.root.join(".codeguard").exists());
    if let Some(path) = std::env::var_os("CODEGUARD_TEST_MAVEN_UNBOUND_EVIDENCE") {
        fs::write(path, serde_json::to_vec_pretty(&report).unwrap()).unwrap();
    }
}
