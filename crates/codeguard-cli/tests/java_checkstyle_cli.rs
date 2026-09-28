#![cfg(unix)]

use std::process::Command;

#[test]
fn checkstyle_missing_prerequisites_is_structured_not_usage_or_clean() {
    let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args([
            "lint",
            "java",
            "Missing.java",
            "--checker=checkstyle",
            "--format=json",
        ])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(3));
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["report_type"], "java_checkstyle_local_feedback");
    assert_eq!(report["reason"], "prerequisites_missing");
    assert_eq!(report["delivery_decision"], "not_evaluated");
    assert_eq!(report["coverage_proven"], false);
    assert_eq!(report["schema_version"], "0.4.0");
    assert!(report["findings"].as_array().unwrap().is_empty());
}

#[test]
fn duplicated_or_unknown_options_are_usage_errors() {
    for tail in [
        vec!["--config=a", "--config=b"],
        vec!["--unknown=a"],
        vec!["--format=yaml"],
    ] {
        let mut args = vec!["lint", "java", "Foo.java", "--checker=checkstyle"];
        args.extend(tail);
        assert_eq!(
            Command::new(env!("CARGO_BIN_EXE_codeguard"))
                .args(args)
                .output()
                .unwrap()
                .status
                .code(),
            Some(2)
        );
    }
}

#[test]
#[ignore = "requires explicit Java and Checkstyle 10.21.4 all.jar"]
fn real_checkstyle_cli_displays_diagnostics_and_never_certifies_project() {
    let root = std::env::temp_dir()
        .canonicalize()
        .unwrap()
        .join(format!("cg-checkstyle-cli-test-{}", std::process::id()));
    std::fs::create_dir(&root).unwrap();
    let source = root.join("Foo.java");
    let config = root.join("checks.xml");
    std::fs::write(&source, "public class Foo {}\n").unwrap();
    std::fs::write(&config,"<!DOCTYPE module PUBLIC \"-//Checkstyle//DTD Checkstyle Configuration 1.3//EN\" \"https://checkstyle.org/dtds/configuration_1_3.dtd\"><module name=\"Checker\"><module name=\"TreeWalker\"><module name=\"MissingJavadocType\"><property name=\"id\" value=\"publicType\"/></module></module></module>").unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args([
            "lint",
            "java",
            source.to_str().unwrap(),
            "--checker=checkstyle",
            "--config",
            config.to_str().unwrap(),
            "--java-tool",
            &std::env::var("CODEGUARD_JAVA_BIN").unwrap(),
            "--checkstyle-jar",
            &std::env::var("CODEGUARD_CHECKSTYLE_JAR").unwrap(),
            "--format=json",
        ])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(3));
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["local_status"], "observed", "{report}");
    assert_eq!(report["findings"].as_array().unwrap().len(), 1);
    assert_eq!(report["findings"][0]["rule_id"], "publicType");
    assert_eq!(
        report["findings"][0]["checker_class"],
        "com.puppycrawl.tools.checkstyle.checks.javadoc.MissingJavadocTypeCheck"
    );
    assert_eq!(report["findings"][0]["rule_summary"], "类型缺少 Javadoc");
    assert_eq!(
        report["findings"][0]["repair_steps"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
    assert_eq!(report["recheck_argv"][0], "codeguard");
    assert_eq!(report["delivery_decision"], "not_evaluated");
    assert_eq!(report["coverage_proven"], false);
    std::fs::write(
        &source,
        "public class Foo {\n/** Calculates. */\npublic int get(int value) { return value; }\n}\n",
    )
    .unwrap();
    for shared in [true, false] {
        let method_id = if shared { "publicType" } else { "publicMethod" };
        std::fs::write(&config,format!("<!DOCTYPE module PUBLIC \"-//Checkstyle//DTD Checkstyle Configuration 1.3//EN\" \"https://checkstyle.org/dtds/configuration_1_3.dtd\"><module name=\"Checker\"><module name=\"TreeWalker\"><module name=\"MissingJavadocType\"><property name=\"id\" value=\"publicType\"/></module><module name=\"JavadocMethod\"><property name=\"id\" value=\"{method_id}\"/></module></module></module>")).unwrap();
        let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
            .args([
                "lint",
                "java",
                source.to_str().unwrap(),
                "--checker=checkstyle",
                "--config",
                config.to_str().unwrap(),
                "--java-tool",
                &std::env::var("CODEGUARD_JAVA_BIN").unwrap(),
                "--checkstyle-jar",
                &std::env::var("CODEGUARD_CHECKSTYLE_JAR").unwrap(),
                "--format=json",
            ])
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(3));
        let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
        if shared {
            assert_eq!(
                report["reason"],
                "checkstyle_configuration_context_unresolved"
            );
            assert!(report["findings"].as_array().unwrap().is_empty());
        } else {
            assert_eq!(report["local_status"], "observed", "{report}");
            assert_eq!(report["findings"].as_array().unwrap().len(), 3);
            assert!(
                report["findings"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|d| d["rule_id"] == "publicMethod")
            );
        }
    }
    std::fs::remove_dir_all(root).unwrap();
}
