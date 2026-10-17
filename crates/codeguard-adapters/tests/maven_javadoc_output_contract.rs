use codeguard_adapters::{
    MavenJavadocParseState, parse_detailed_maven_javadoc_output, parse_maven_javadoc_output,
};
use std::collections::BTreeMap;

const SOURCE: &[u8] = b"package demo;\npublic class Bad {}\n";
const WARNING_BLOCK: &str = "[INFO] --- javadoc:3.12.0:javadoc (default-cli) @ demo ---\n[WARNING] Javadoc Warnings\n[WARNING] /workspace/src/main/java/demo/Bad.java:2: warning: no comment\n[WARNING] public class Bad {}\n[WARNING] ^\n[WARNING] 1 warning\n";

fn sources() -> BTreeMap<String, Vec<u8>> {
    BTreeMap::from([(
        "/workspace/src/main/java/demo/Bad.java".into(),
        SOURCE.to_vec(),
    )])
}

#[test]
fn build_success_with_native_warning_is_not_clean() {
    let log = format!("{WARNING_BLOCK}[INFO] BUILD SUCCESS\n");
    let parsed = parse_maven_javadoc_output(log.as_bytes(), 0, "3.12.0", &sources());
    assert_eq!(parsed.state, MavenJavadocParseState::ValidDiagnostics);
    assert_eq!(parsed.diagnostics.len(), 1);
    assert_eq!(parsed.diagnostics[0].rule_id, "JavadocMissingComment");
    assert_eq!(
        parsed.diagnostics[0].path,
        "/workspace/src/main/java/demo/Bad.java"
    );
}

#[test]
fn warning_failure_is_a_diagnostic_not_an_environment_error() {
    let log = format!(
        "{WARNING_BLOCK}[INFO] BUILD FAILURE\n[ERROR] Failed to execute goal org.apache.maven.plugins:maven-javadoc-plugin:3.12.0:javadoc (default-cli) on project demo: An error has occurred in Javadoc report generation: Project contains Javadoc Warnings -> [Help 1]\n"
    );
    let parsed = parse_maven_javadoc_output(log.as_bytes(), 1, "3.12.0", &sources());
    assert_eq!(parsed.state, MavenJavadocParseState::ValidDiagnostics);
    assert_eq!(parsed.diagnostics.len(), 1);
}

#[test]
fn clean_success_log_cannot_prove_a_fresh_javadoc_execution() {
    let log = "[INFO] --- javadoc:3.12.0:javadoc (default-cli) @ demo ---\n[WARNING] Source files encoding has not been set, using platform encoding UTF-8, i.e. build is platform dependent!\n[INFO] BUILD SUCCESS\n";
    for _ in 0..2 {
        let parsed = parse_maven_javadoc_output(log.as_bytes(), 0, "3.12.0", &sources());
        assert_eq!(parsed.state, MavenJavadocParseState::CleanLogUnverified);
        assert_eq!(parsed.reason, Some("javadoc_fresh_execution_unverified"));
        assert!(parsed.diagnostics.is_empty());
    }
}

#[test]
fn unknown_warning_or_failed_clean_log_is_incomplete() {
    let base = "[INFO] --- javadoc:3.12.0:javadoc (default-cli) @ demo ---\n";
    for (log, exit) in [
        (
            format!("{base}[WARNING] Unknown Javadoc warning\n[INFO] BUILD SUCCESS\n"),
            0,
        ),
        (
            format!("{base}[WARNING] 1 warning\n[INFO] BUILD SUCCESS\n"),
            0,
        ),
        (format!("{base}[INFO] BUILD FAILURE\n"), 1),
        (
            format!("{base}[INFO] BUILD SUCCESS\n[ERROR] Unrelated failure\n"),
            0,
        ),
    ] {
        let parsed = parse_maven_javadoc_output(log.as_bytes(), exit, "3.12.0", &sources());
        assert_eq!(parsed.state, MavenJavadocParseState::Incomplete, "{log}");
        assert!(parsed.diagnostics.is_empty());
    }
}

#[test]
fn unknown_warning_or_mismatched_source_is_incomplete_without_findings() {
    let unknown = WARNING_BLOCK.replace("warning: no comment", "warning: maybe no comment");
    let parsed = parse_maven_javadoc_output(
        format!("{unknown}[INFO] BUILD SUCCESS\n").as_bytes(),
        0,
        "3.12.0",
        &sources(),
    );
    assert_eq!(parsed.state, MavenJavadocParseState::Incomplete);
    assert!(parsed.diagnostics.is_empty());

    let mismatched = WARNING_BLOCK.replace("public class Bad {}", "public class Other {}");
    let parsed = parse_maven_javadoc_output(
        format!("{mismatched}[INFO] BUILD SUCCESS\n").as_bytes(),
        0,
        "3.12.0",
        &sources(),
    );
    assert_eq!(parsed.state, MavenJavadocParseState::Incomplete);
    assert!(parsed.diagnostics.is_empty());

    let mixed_failure = format!(
        "{WARNING_BLOCK}[INFO] BUILD FAILURE\n[ERROR] Failed to execute goal org.apache.maven.plugins:maven-javadoc-plugin:3.12.0:javadoc (default-cli) on project demo: Project contains Javadoc Warnings -> [Help 1]\n[ERROR] Compilation failed\n"
    );
    let parsed = parse_maven_javadoc_output(mixed_failure.as_bytes(), 1, "3.12.0", &sources());
    assert_eq!(parsed.state, MavenJavadocParseState::Incomplete);
    assert!(parsed.diagnostics.is_empty());
}

#[test]
#[ignore = "requires explicit Maven 3.x, JDK 21 and cached Javadoc plugin 3.12.0"]
fn real_maven_success_and_failure_both_preserve_native_diagnostics() {
    use std::fs;
    use std::process::Command;
    use std::sync::atomic::{AtomicU64, Ordering};
    static NEXT: AtomicU64 = AtomicU64::new(0);

    let maven = std::env::var("CODEGUARD_MAVEN_BIN").unwrap();
    let java_home = std::env::var("CODEGUARD_JAVA_HOME").unwrap();
    for (fail_on_warnings, expected_exit) in [(false, 0), (true, 1)] {
        let root = std::env::temp_dir().canonicalize().unwrap().join(format!(
            "cg-maven-javadoc-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&root).unwrap();
        let source = root.join("src/main/java/demo/Bad.java");
        fs::create_dir_all(source.parent().unwrap()).unwrap();
        let bytes = b"package demo;\npublic class Bad {}\n";
        fs::write(&source, bytes).unwrap();
        let sources = BTreeMap::from([(source.to_string_lossy().into_owned(), bytes.to_vec())]);
        let pom = format!(
            "<project xmlns=\"http://maven.apache.org/POM/4.0.0\"><modelVersion>4.0.0</modelVersion><groupId>demo</groupId><artifactId>javadoc-probe</artifactId><version>1</version><build><plugins><plugin><groupId>org.apache.maven.plugins</groupId><artifactId>maven-javadoc-plugin</artifactId><version>3.12.0</version><configuration><doclint>missing</doclint><failOnWarnings>{fail_on_warnings}</failOnWarnings><failOnError>true</failOnError></configuration></plugin></plugins></build></project>"
        );
        fs::write(root.join("pom.xml"), pom).unwrap();
        let output = Command::new(&maven)
            .args([
                "-B",
                "-o",
                "-f",
                root.join("pom.xml").to_str().unwrap(),
                "org.apache.maven.plugins:maven-javadoc-plugin:3.12.0:javadoc",
            ])
            .env("JAVA_HOME", &java_home)
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(expected_exit));
        let parsed = parse_maven_javadoc_output(&output.stdout, expected_exit, "3.12.0", &sources);
        assert_eq!(
            parsed.state,
            MavenJavadocParseState::ValidDiagnostics,
            "{parsed:?}\n{}",
            String::from_utf8_lossy(&output.stdout)
        );
        assert!(
            parsed
                .diagnostics
                .iter()
                .any(|item| item.rule_id == "JavadocMissingComment")
        );
        fs::remove_dir_all(root).unwrap();
    }
}

#[test]
#[ignore = "requires explicit Maven 3.x, JDK 21 and cached Javadoc plugin 3.12.0"]
fn real_maven_clean_and_repeated_logs_remain_unverified() {
    use std::fs;
    use std::process::Command;

    let maven = std::env::var("CODEGUARD_MAVEN_BIN").unwrap();
    let java_home = std::env::var("CODEGUARD_JAVA_HOME").unwrap();
    let root = std::env::temp_dir().canonicalize().unwrap().join(format!(
        "cg-maven-javadoc-clean-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir(&root).unwrap();
    let source = root.join("src/main/java/demo/Good.java");
    fs::create_dir_all(source.parent().unwrap()).unwrap();
    let bytes = b"package demo;\n/** Documented class. */\npublic class Good {\n    /** Creates a documented instance. */\n    public Good() {}\n}\n";
    fs::write(&source, bytes).unwrap();
    fs::write(
        root.join("pom.xml"),
        "<project xmlns=\"http://maven.apache.org/POM/4.0.0\"><modelVersion>4.0.0</modelVersion><groupId>demo</groupId><artifactId>javadoc-probe</artifactId><version>1</version><build><plugins><plugin><groupId>org.apache.maven.plugins</groupId><artifactId>maven-javadoc-plugin</artifactId><version>3.12.0</version><configuration><doclint>missing</doclint><failOnWarnings>true</failOnWarnings><failOnError>true</failOnError></configuration></plugin></plugins></build></project>",
    )
    .unwrap();
    let sources = BTreeMap::from([(source.to_string_lossy().into_owned(), bytes.to_vec())]);
    for _ in 0..2 {
        let output = Command::new(&maven)
            .args([
                "-B",
                "-o",
                "-f",
                root.join("pom.xml").to_str().unwrap(),
                "org.apache.maven.plugins:maven-javadoc-plugin:3.12.0:javadoc",
            ])
            .env("JAVA_HOME", &java_home)
            .output()
            .unwrap();
        assert_eq!(
            output.status.code(),
            Some(0),
            "{}",
            String::from_utf8_lossy(&output.stdout)
        );
        let parsed = parse_maven_javadoc_output(&output.stdout, 0, "3.12.0", &sources);
        assert_eq!(
            parsed.state,
            MavenJavadocParseState::CleanLogUnverified,
            "{parsed:?}\n{}",
            String::from_utf8_lossy(&output.stdout)
        );
    }
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn detailed_descriptions_survive_maven_success_and_warning_failure() {
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
            let block =
                WARNING_BLOCK.replace("warning: no comment", &format!("warning: {message}"));
            let footer = if exit == 0 {
                "[INFO] BUILD SUCCESS\n"
            } else {
                "[INFO] BUILD FAILURE\n[ERROR] Failed to execute goal org.apache.maven.plugins:maven-javadoc-plugin:3.12.0:javadoc (default-cli) on project demo: Project contains Javadoc Warnings -> [Help 1]\n"
            };
            let bytes = format!("{block}{footer}");
            assert_eq!(
                parse_maven_javadoc_output(bytes.as_bytes(), exit, "3.12.0", &sources()).state,
                MavenJavadocParseState::Incomplete
            );
            let parsed =
                parse_detailed_maven_javadoc_output(bytes.as_bytes(), exit, "3.12.0", &sources());
            assert_eq!(
                parsed.state,
                MavenJavadocParseState::ValidDiagnostics,
                "{message}"
            );
            assert_eq!(parsed.diagnostics[0].rule_id, rule);
        }
    }
}

#[test]
fn detailed_parser_rejects_wrong_source_and_unknown_messages_without_findings() {
    for log in [
        WARNING_BLOCK.replace("warning: no comment", "warning: unknown detailed rule"),
        WARNING_BLOCK
            .replace("warning: no comment", "warning: empty comment")
            .replace("public class Bad {}", "public class Other {}"),
    ] {
        let parsed = parse_detailed_maven_javadoc_output(
            format!("{log}[INFO] BUILD SUCCESS\n").as_bytes(),
            0,
            "3.12.0",
            &sources(),
        );
        assert_eq!(parsed.state, MavenJavadocParseState::Incomplete);
        assert!(parsed.diagnostics.is_empty());
    }
}

#[test]
fn offline_missing_plugin_requires_the_exact_failure_and_preserves_legacy_contract() {
    let log = "[INFO] Scanning for projects...\n[INFO] BUILD FAILURE\n[ERROR] Plugin org.apache.maven.plugins:maven-javadoc-plugin:3.12.0 or one of its dependencies could not be resolved:\n[ERROR] \tCannot access central (https://repo.maven.apache.org/maven2) in offline mode and the artifact org.apache.maven.plugins:maven-javadoc-plugin:jar:3.12.0 has not been downloaded from it before.\n";
    let parsed = parse_detailed_maven_javadoc_output(log.as_bytes(), 1, "3.12.0", &sources());
    assert_eq!(parsed.state, MavenJavadocParseState::Incomplete);
    assert_eq!(
        parsed.reason,
        Some("maven_javadoc_offline_plugin_unavailable")
    );
    assert!(parsed.diagnostics.is_empty());
    assert_eq!(
        parse_maven_javadoc_output(log.as_bytes(), 1, "3.12.0", &sources()).reason,
        Some("javadoc_goal_unverified")
    );
    for (bad, exit) in [
        (log.to_string(), 0),
        (log.replace("jar:3.12.0", "jar:3.11.0"), 1),
        (log.replace(" in offline mode", " online"), 1),
        (format!("{log}[INFO] BUILD SUCCESS\n"), 1),
    ] {
        let parsed =
            parse_detailed_maven_javadoc_output(bad.as_bytes(), exit, "3.12.0", &sources());
        assert_ne!(
            parsed.reason,
            Some("maven_javadoc_offline_plugin_unavailable")
        );
        assert!(parsed.diagnostics.is_empty());
    }
}
