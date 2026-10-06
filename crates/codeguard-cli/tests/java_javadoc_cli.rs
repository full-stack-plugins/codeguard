#![cfg(unix)]

use serde_json::Value;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

struct Fixture {
    root: PathBuf,
    source: PathBuf,
    java_home: PathBuf,
}

impl Fixture {
    fn new(launcher: &str) -> Self {
        let id = NEXT.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir()
            .canonicalize()
            .unwrap()
            .join(format!("cg-javadoc-cli-{}-{id}", std::process::id()));
        fs::create_dir(&root).unwrap();
        let source = root.join("Bad.java");
        fs::write(&source, "public class Bad {}\n").unwrap();
        let java_home = root.join("jdk");
        fs::create_dir_all(java_home.join("bin")).unwrap();
        fs::write(java_home.join("bin/java"), b"fixture java").unwrap();
        fs::write(java_home.join("release"), b"JAVA_VERSION=\"21.0.12\"\n").unwrap();
        let tool = java_home.join("bin/javadoc");
        fs::write(&tool, launcher).unwrap();
        fs::set_permissions(&tool, fs::Permissions::from_mode(0o700)).unwrap();
        Self {
            root,
            source,
            java_home,
        }
    }

    fn run(&self) -> (i32, Value) {
        let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
            .args([
                "lint",
                "java",
                self.source.to_str().unwrap(),
                "--checker",
                "javadoc",
                "--java-home",
                self.java_home.to_str().unwrap(),
                "--format=json",
            ])
            .output()
            .unwrap();
        assert!(
            output.stderr.is_empty(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        (
            output.status.code().unwrap(),
            serde_json::from_slice(&output.stdout).unwrap(),
        )
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.root).unwrap();
    }
}

#[test]
fn native_javadoc_warning_becomes_untrusted_comment_diagnostic() {
    let fixture = Fixture::new(
        "#!/bin/sh\nmkdir -p docs\nprintf '<html></html>' > docs/index.html\nprintf '%s:1: warning: no comment\\npublic class Bad {}\\n       ^\\n1 warning\\n' \"$PWD/src/Bad.java\" >&2\n",
    );
    let (exit, report) = fixture.run();
    assert_eq!(exit, 3);
    assert_eq!(report["report_type"], "java_javadoc_local_feedback");
    assert_eq!(report["local_status"], "findings_observed_untrusted");
    assert_eq!(report["findings"][0]["rule_id"], "JavadocMissingComment");
    assert_eq!(report["findings"][0]["column"], 8);
    assert_eq!(report["coverage_proven"], false);
    assert_eq!(report["delivery_decision"], "not_evaluated");
    assert_eq!(fs::read(&fixture.source).unwrap(), b"public class Bad {}\n");
}

#[test]
fn unknown_javadoc_output_is_incomplete_without_source_finding() {
    let fixture = Fixture::new(
        "#!/bin/sh\nmkdir -p docs\nprintf '<html></html>' > docs/index.html\nprintf '%s:1: warning: unknown diagnostic\\npublic class Bad {}\\n       ^\\n1 warning\\n' \"$PWD/src/Bad.java\" >&2\n",
    );
    let (exit, report) = fixture.run();
    assert_eq!(exit, 3);
    assert_eq!(report["local_status"], "incomplete");
    assert_eq!(report["reason"], "javadoc_rule_unrecognized");
    assert!(report["findings"].as_array().unwrap().is_empty());
}

#[test]
fn missing_jdk_version_or_document_output_cannot_be_clean() {
    let missing_version = Fixture::new("#!/bin/sh\nexit 0\n");
    fs::remove_file(missing_version.java_home.join("release")).unwrap();
    let (_, unavailable) = missing_version.run();
    assert_eq!(unavailable["reason"], "jdk_version_unverified");
    assert_eq!(unavailable["local_status"], "incomplete");

    let missing_output = Fixture::new("#!/bin/sh\nexit 0\n");
    let (_, output) = missing_output.run();
    assert_eq!(output["reason"], "native_output_missing");
    assert_eq!(output["local_status"], "incomplete");
}

#[test]
fn invalid_checker_selection_is_usage_error_before_native_execution() {
    let fixture = Fixture::new("#!/bin/sh\nexit 0\n");
    for args in [
        vec!["--checker", "unknown"],
        vec!["--checker", "javadoc", "--checker", "p3c"],
        vec!["--checker"],
    ] {
        let mut command = Command::new(env!("CARGO_BIN_EXE_codeguard"));
        command.args(["lint", "java", fixture.source.to_str().unwrap()]);
        command.args(args);
        let output = command.output().unwrap();
        assert_eq!(output.status.code(), Some(2));
        assert!(output.stdout.is_empty());
    }
}

#[test]
#[ignore = "requires an explicit JDK 21 installation"]
fn real_jdk21_javadoc_missing_and_documented_examples() {
    let home = std::env::var("CODEGUARD_JAVA_HOME").unwrap();
    let id = NEXT.fetch_add(1, Ordering::Relaxed);
    let root = std::env::temp_dir()
        .canonicalize()
        .unwrap()
        .join(format!("cg-javadoc-native-{}-{id}", std::process::id()));
    fs::create_dir(&root).unwrap();
    let source = root.join("Bad.java");
    fs::write(&source, "public class Bad {}\n").unwrap();
    let run = |source: &PathBuf| {
        let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
            .args([
                "lint",
                "java",
                source.to_str().unwrap(),
                "--checker",
                "javadoc",
                "--java-home",
                &home,
                "--format=json",
            ])
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(3));
        serde_json::from_slice::<Value>(&output.stdout).unwrap()
    };
    let missing = run(&source);
    assert_eq!(
        missing["local_status"], "findings_observed_untrusted",
        "{missing}"
    );
    assert!(
        missing["findings"]
            .as_array()
            .unwrap()
            .iter()
            .any(|finding| finding["rule_id"] == "JavadocMissingComment")
    );
    fs::write(&source, "/** Documented class. */\npublic class Bad { /** Documented constructor. */ public Bad() {} }\n").unwrap();
    let documented = run(&source);
    assert_eq!(
        documented["local_status"], "clean_scope_unproven",
        "{documented}"
    );
    assert!(documented["findings"].as_array().unwrap().is_empty());
    fs::write(&source, "/** Class documentation. */\npublic class Bad {\n  /** Documented constructor. */ public Bad() {}\n  /** Run action. */ public int run(int value) { return value; }\n}\n").unwrap();
    let tags = run(&source);
    assert_eq!(
        tags["local_status"], "findings_observed_untrusted",
        "{tags}"
    );
    for rule in ["JavadocMissingParam", "JavadocMissingReturn"] {
        assert!(
            tags["findings"]
                .as_array()
                .unwrap()
                .iter()
                .any(|finding| finding["rule_id"] == rule),
            "{tags}"
        );
    }
    fs::write(&source, "/** Implementation. */\npublic class Bad implements Runnable {\n  /** Constructor. */ public Bad() {}\n  /** {@inheritDoc} */ public void run() {}\n}\n").unwrap();
    let inherited = run(&source);
    assert_eq!(
        inherited["local_status"], "clean_scope_unproven",
        "{inherited}"
    );
    fs::write(
        &source,
        "/** Record documentation. */\npublic record Bad(int value) {}\n",
    )
    .unwrap();
    let record = run(&source);
    assert_eq!(
        record["local_status"], "findings_observed_untrusted",
        "{record}"
    );
    assert!(
        record["findings"]
            .as_array()
            .unwrap()
            .iter()
            .any(|finding| finding["rule_id"] == "JavadocMissingParam")
    );
    fs::write(&source, "/**\n * Record documentation.\n * @param value stored value\n */\npublic record Bad(int value) {}\n").unwrap();
    let documented_record = run(&source);
    assert_eq!(
        documented_record["local_status"], "clean_scope_unproven",
        "{documented_record}"
    );
    fs::write(&source, "import lombok.Data;\n/** Lombok model. */\n@Data public class Bad { private int value; }\n").unwrap();
    let unresolved_classpath = run(&source);
    assert_eq!(
        unresolved_classpath["local_status"], "incomplete",
        "{unresolved_classpath}"
    );
    assert!(
        unresolved_classpath["findings"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn detailed_native_warning_is_not_an_unknown_diagnostic() {
    let fixture = Fixture::new(
        "#!/bin/sh\nmkdir -p docs\nprintf '<html></html>' > docs/index.html\nprintf '%s:1: warning: empty comment\\npublic class Bad {}\\n       ^\\n1 warning\\n' \"$PWD/src/Bad.java\" >&2\n",
    );
    let (_, report) = fixture.run();
    assert_eq!(report["schema_version"], "0.2.0");
    assert_eq!(report["local_status"], "findings_observed_untrusted");
    assert_eq!(report["findings"][0]["rule_id"], "JavadocEmptyComment");
}
