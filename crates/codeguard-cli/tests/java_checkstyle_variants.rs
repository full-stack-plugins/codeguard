#![cfg(unix)]

use serde_json::Value;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;

struct Fixture(PathBuf);
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn fixture(label: &str) -> Fixture {
    let root = std::env::temp_dir().canonicalize().unwrap().join(format!(
        "cg-checkstyle-variants-{}-{label}",
        std::process::id()
    ));
    fs::create_dir(&root).unwrap();
    fs::set_permissions(&root, fs::Permissions::from_mode(0o700)).unwrap();
    fs::write(root.join("checks.xml"),"<!DOCTYPE module PUBLIC \"-//Checkstyle//DTD Checkstyle Configuration 1.3//EN\" \"https://checkstyle.org/dtds/configuration_1_3.dtd\"><module name=\"Checker\"><module name=\"TreeWalker\"><module name=\"MissingJavadocType\"><property name=\"id\" value=\"missingType\"/></module><module name=\"JavadocType\"><property name=\"id\" value=\"typeTags\"/></module><module name=\"MissingJavadocMethod\"><property name=\"id\" value=\"missingMethod\"/></module><module name=\"JavadocMethod\"><property name=\"id\" value=\"methodTags\"/></module></module></module>").unwrap();
    Fixture(root)
}

fn check(root: &Path, source: &Path) -> Value {
    let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args([
            "lint",
            "java",
            source.to_str().unwrap(),
            "--checker=checkstyle",
            "--config",
            root.join("checks.xml").to_str().unwrap(),
            "--java-tool",
            &std::env::var("CODEGUARD_JAVA_BIN").unwrap(),
            "--checkstyle-jar",
            &std::env::var("CODEGUARD_CHECKSTYLE_JAR").unwrap(),
            "--format=json",
        ])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(3));
    let report: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["local_status"], "observed", "{report}");
    assert_eq!(report["coverage_proven"], false);
    assert_eq!(report["delivery_decision"], "not_evaluated");
    report
}

#[test]
#[ignore = "requires explicit Java and Checkstyle 10.21.4"]
fn real_record_inherited_and_generated_comment_boundaries() {
    let root = fixture("comments");
    let source = root.0.join("Foo.java");
    let cases = [
        (
            "/** Record. */\npublic record Foo(int value) {}\n",
            Some(("typeTags", 1)),
        ),
        (
            "/** Record.\n * @param value input\n */\npublic record Foo(int value) {}\n",
            None,
        ),
        (
            "/** API. */\npublic class Foo { public int get(int value) {return value;} }\n",
            Some(("methodTags", 2)),
        ),
        (
            "/** API. */\npublic class Foo {\npublic int get(int value) {return value;}\n}\n",
            None,
        ),
        (
            "/** API. */\npublic class Foo {\npublic int get(int value) {\nreturn value;\n}\n}\n",
            Some(("missingMethod", 1)),
        ),
        (
            "/** Base API. */\ninterface Base { /** Computes.\n * @param value input\n * @return input\n */\nint get(int value); }\n/** API. */\npublic class Foo implements Base { /** {@inheritDoc} */\n@Override public int get(int value) { return value; } }\n",
            None,
        ),
        (
            "/** API. */\npublic class Foo implements Runnable { @Override public void run() {} }\n",
            None,
        ),
        (
            "import javax.annotation.processing.Generated;\n@Generated(\"fixture\")\npublic class Foo {}\n",
            None,
        ),
        ("public class Foo {}\n", Some(("missingType", 1))),
    ];
    for (text, rule) in cases {
        fs::write(&source, text).unwrap();
        let report = check(&root.0, &source);
        let findings = report["findings"].as_array().unwrap();
        if let Some((rule, count)) = rule {
            assert_eq!(findings.len(), count, "{report}");
            assert!(findings.iter().all(|d| d["rule_id"] == rule), "{report}");
        } else {
            assert!(findings.is_empty(), "{report}");
        }
    }
}

#[test]
#[ignore = "requires explicit Java, Checkstyle 10.21.4, and local Lombok jar"]
fn real_lombok_source_and_delombok_are_distinct_scopes() {
    use codeguard_runtime::{ProcessSpec, Termination, run_process_recorded};
    use std::collections::BTreeMap;
    use std::sync::atomic::AtomicBool;
    use std::time::{Duration, Instant};
    let root = fixture("lombok");
    let source = root.0.join("Foo.java");
    fs::write(
        &source,
        "import lombok.Getter;\n/** API. */\n@Getter public class Foo { private int value; }\n",
    )
    .unwrap();
    let original = check(&root.0, &source);
    assert!(original["findings"].as_array().unwrap().is_empty());
    let out = root.0.join("generated");
    let spec = ProcessSpec {
        executable: PathBuf::from(std::env::var_os("CODEGUARD_JAVA_BIN").unwrap()),
        args: vec![
            "-jar".into(),
            std::env::var_os("CODEGUARD_LOMBOK_JAR").unwrap(),
            "delombok".into(),
            source.as_os_str().into(),
            "-d".into(),
            out.as_os_str().into(),
        ],
        cwd: root.0.clone(),
        env: BTreeMap::new(),
        stdin: None,
        deadline: Instant::now() + Duration::from_secs(20),
        output_limit_bytes: 1024 * 1024,
    };
    let result =
        run_process_recorded(&spec, &AtomicBool::new(false), &root.0, "delombok.log").unwrap();
    assert_eq!(result.termination, Termination::Exited(0));
    let generated = out.join("Foo.java");
    let text = fs::read_to_string(&generated).unwrap();
    assert!(text.contains("getValue()"));
    let report = check(&root.0, &generated);
    assert_eq!(report["findings"].as_array().unwrap().len(), 1, "{report}");
    assert_eq!(report["findings"][0]["rule_id"], "missingMethod");
    assert_eq!(report["findings"][0]["path"], generated.to_str().unwrap());
}
