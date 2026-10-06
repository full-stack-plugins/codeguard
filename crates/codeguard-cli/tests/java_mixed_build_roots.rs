#![cfg(unix)]
//! Maven/Gradle共存时保留各原生路径；静态配置不能覆盖另一构建器的未解析义务。
use serde_json::{Value, json};
use std::{
    fs,
    path::PathBuf,
    process::Command,
    sync::atomic::{AtomicU64, Ordering},
};
static NEXT: AtomicU64 = AtomicU64::new(0);
struct Project(PathBuf);
impl Project {
    fn new() -> Self {
        let root = std::env::temp_dir().canonicalize().unwrap().join(format!(
            "cg-java-mixed-build-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(root.join("src/main/java")).unwrap();
        fs::write(root.join("src/main/java/A.java"), b"class A {}\n").unwrap();
        Self(root)
    }
    fn run(&self, args: &[&str]) -> Value {
        let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
            .args(args)
            .arg(&self.0)
            .arg("--format=json")
            .env("PATH", "/no/tools")
            .output()
            .unwrap();
        assert!(matches!(output.status.code(), Some(0 | 3)), "{output:?}");
        let report = serde_json::from_slice(&output.stdout).unwrap();
        if let Some(dir) = std::env::var_os("CODEGUARD_JAVA_MIXED_BUILD_REPORT_DIR") {
            fs::create_dir_all(&dir).unwrap();
            fs::write(
                PathBuf::from(dir).join(format!(
                    "{}-{}.json",
                    std::process::id(),
                    NEXT.fetch_add(1, Ordering::Relaxed)
                )),
                &output.stdout,
            )
            .unwrap();
        }
        report
    }
}
impl Drop for Project {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
const POM: &str = r#"<project><modelVersion>4.0.0</modelVersion><groupId>demo</groupId><artifactId>app</artifactId><version>1</version><build><plugins><plugin><groupId>org.apache.maven.plugins</groupId><artifactId>maven-dependency-plugin</artifactId><version>3.8.1</version></plugin><plugin><groupId>org.owasp</groupId><artifactId>dependency-check-maven</artifactId><version>12.1.0</version></plugin></plugins></build></project>"#;
#[test]
fn same_root_maven_and_each_gradle_dsl_preserve_both_checker_paths() {
    for name in ["build.gradle", "build.gradle.kts"] {
        let p = Project::new();
        fs::write(p.0.join("pom.xml"), POM).unwrap();
        fs::write(
            p.0.join(name),
            b"plugins { id(\"org.owasp.dependencycheck\") version \"13.0.0\" }\n",
        )
        .unwrap();
        let detected = p.run(&["detect"]);
        let rows = detected["checker_configurations"].as_array().unwrap();
        for (id, source, status) in [
            ("java.maven.dependency_check", "pom.xml", "configured"),
            ("java.gradle.dependency_check", name, "unknown"),
        ] {
            assert!(
                rows.iter().any(|row| row["checker_id"] == id
                    && row["configuration_ref"] == source
                    && row["configuration"] == status),
                "{detected}"
            );
        }
        let checked = p.run(&["check", "java"]);
        assert_eq!(checked["schema_version"], "0.61.0");
        for category in ["dependencies", "cve", "security"] {
            let row = checked["category_candidates"]
                .as_array()
                .unwrap()
                .iter()
                .find(|row| row["language"] == "java" && row["category"] == category)
                .unwrap();
            assert_eq!(row["checker_id"], Value::Null, "{checked}");
            assert_eq!(row["status"], "configuration_unresolved", "{checked}");
            assert_eq!(row["reason"], "checker_build_systems_mixed", "{checked}");
        }
        let lint = p.run(&["lint", "all"]);
        assert_eq!(lint["schema_version"], "0.61.0");
        assert_eq!(lint["requested_categories"], json!(["lint"]));
        assert!(
            lint["category_candidates"]
                .as_array()
                .unwrap()
                .iter()
                .all(|row| row["category"] == "lint")
        );
        assert_eq!(lint["native_results"]["java_cve"], Value::Null);
        assert_eq!(checked["exit_code"], 3);
        assert_eq!(checked["delivery_decision"], "not_evaluated");
    }
}
#[test]
fn two_gradle_build_files_keep_each_configuration_reference() {
    let p = Project::new();
    for name in ["build.gradle", "build.gradle.kts"] {
        fs::write(p.0.join(name), b"plugins { java }\n").unwrap();
    }
    let detected = p.run(&["detect"]);
    let refs = detected["checker_configurations"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|r| r["checker_id"] == "java.gradle.dependency_check")
        .map(|r| r["configuration_ref"].clone())
        .collect::<Vec<_>>();
    assert_eq!(
        refs,
        json!(["build.gradle", "build.gradle.kts"])
            .as_array()
            .unwrap()
            .clone()
    );
}
#[test]
fn invalid_maven_manifest_does_not_hide_gradle_obligations() {
    let p = Project::new();
    fs::write(p.0.join("pom.xml"), b"<broken>").unwrap();
    fs::write(p.0.join("build.gradle.kts"), b"plugins { java }").unwrap();
    let detected = p.run(&["detect"]);
    let rows = detected["checker_configurations"].as_array().unwrap();
    assert!(rows.iter().any(
        |r| r["checker_id"] == "java.maven.dependency_check" && r["configuration"] == "invalid"
    ));
    assert!(
        rows.iter()
            .any(|r| r["checker_id"] == "java.gradle.dependency_check"
                && r["configuration"] == "unknown")
    );
}
