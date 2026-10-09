//! Actual JDK documentation acceptance. These explicit native tests do not grant
//! project delivery or task-closing authority. Maven/Gradle plugin execution has
//! separate acceptance tests and cannot be inferred from this file probe.

use serde_json::Value;
use std::{
    fs,
    process::Command,
    sync::atomic::{AtomicU64, Ordering},
};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn rules(source: &str) -> Vec<String> {
    let home = std::env::var("CODEGUARD_TEST_JAVA_HOME")
        .expect("explicit JDK 21 is required; missing tools are not a passing observation");
    let root = std::env::temp_dir().join(format!(
        "codeguard-javadoc-acceptance-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir(&root).unwrap();
    let file = root.join("Sample.java");
    fs::write(&file, source).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args([
            "comments",
            "java",
            file.to_str().unwrap(),
            "--java-home",
            &home,
            "--format=json",
        ])
        .env_remove("CODEGUARD_TIMEOUT")
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(3), "{output:?}");
    let report: Value = serde_json::from_slice(&output.stdout).unwrap();
    let observation = &report["native_observation"];
    let findings = observation["findings"]
        .as_array()
        .expect("actual native findings required");
    assert_eq!(
        observation["local_status"],
        if findings.is_empty() {
            "clean_scope_unproven"
        } else {
            "findings_observed_untrusted"
        },
        "{report}"
    );
    assert_eq!(
        observation["javadoc_tool_sha256"].as_str().unwrap().len(),
        64
    );
    let mut rules: Vec<String> = findings
        .iter()
        .map(|f| f["rule_id"].as_str().unwrap().to_owned())
        .collect();
    rules.sort();
    fs::remove_dir_all(root).unwrap();
    rules
}

const COMPLETE: &str = "/** Computes values. */\npublic class Sample {\n/** Creates a calculator. */ public Sample() {}\n/** Stores the result. */ public int value;\n/** Adds two numbers.\n * @param a first operand\n * @param b second operand\n * @return the sum\n */\npublic int add(int a, int b) { return a + b; }\n}\n";

#[test]
#[ignore = "requires explicit CODEGUARD_TEST_JAVA_HOME pointing to JDK 21"]
fn complete_javadoc_passes() {
    assert_eq!(rules(COMPLETE), Vec::<String>::new());
}

#[test]
#[ignore = "requires explicit CODEGUARD_TEST_JAVA_HOME pointing to JDK 21"]
fn missing_type_documentation_detected() {
    assert_eq!(
        rules(&COMPLETE.replace("/** Computes values. */", "")),
        ["JavadocMissingComment"]
    );
}

#[test]
#[ignore = "requires explicit CODEGUARD_TEST_JAVA_HOME pointing to JDK 21"]
fn missing_method_documentation_detected() {
    let start = COMPLETE.find("/** Adds").unwrap();
    let end = COMPLETE[start..].find("*/").unwrap() + start + 2;
    let source = format!("{}{}", &COMPLETE[..start], &COMPLETE[end..]);
    assert_eq!(rules(&source), ["JavadocMissingComment"]);
}

#[test]
#[ignore = "requires explicit CODEGUARD_TEST_JAVA_HOME pointing to JDK 21"]
fn missing_field_documentation_detected() {
    assert_eq!(
        rules(&COMPLETE.replace("/** Stores the result. */", "")),
        ["JavadocMissingComment"]
    );
}

#[test]
#[ignore = "requires explicit CODEGUARD_TEST_JAVA_HOME pointing to JDK 21"]
fn bare_tag_cannot_impersonate_compliance() {
    let source = COMPLETE
        .replace("a first operand", "a")
        .replace("b second operand", "b")
        .replace("@return the sum", "@return");
    assert_eq!(
        rules(&source),
        [
            "JavadocEmptyParamDescription",
            "JavadocEmptyParamDescription",
            "JavadocEmptyReturnDescription"
        ]
    );
}

#[test]
#[ignore = "requires explicit CODEGUARD_TEST_JAVA_HOME pointing to JDK 21"]
fn line_comment_cannot_impersonate_javadoc() {
    assert_eq!(
        rules(&COMPLETE.replace("/** Computes values. */", "// Computes values.")),
        ["JavadocMissingComment"]
    );
}

#[test]
#[ignore = "requires explicit CODEGUARD_TEST_JAVA_HOME pointing to JDK 21"]
fn missing_param_tag_detected() {
    assert_eq!(
        rules(&COMPLETE.replace(" * @param a first operand\n", "")),
        ["JavadocMissingParam"]
    );
}

#[test]
#[ignore = "requires explicit CODEGUARD_TEST_JAVA_HOME pointing to JDK 21"]
fn missing_return_tag_detected() {
    assert_eq!(
        rules(&COMPLETE.replace(" * @return the sum\n", "")),
        ["JavadocMissingReturn"]
    );
}
