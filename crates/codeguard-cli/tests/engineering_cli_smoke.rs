#![cfg(unix)]
//! 真实已编译CLI的低成本采集；不安装工具、不运行全语言矩阵。
use codeguard_cli::engineering_projection::project_raw;
use codeguard_core::DeliveryDecision;
use std::{fs, path::PathBuf, process::Command};
struct Fixture(PathBuf);
impl Drop for Fixture {
    fn drop(&mut self) {
        if self.0.parent() == Some(std::env::temp_dir().as_path()) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }
}
#[test]
fn real_java_and_all_cli_missing_tools_preserve_raw_feedback_without_delivery_upgrade() {
    let dir = std::env::temp_dir().join(format!(
        "codeguard-engineering-smoke-{}",
        std::process::id()
    ));
    fs::create_dir(&dir).expect("fresh isolated fixture; never overwrite an existing directory");
    let fixture = Fixture(dir);
    fs::create_dir_all(fixture.0.join("src/main/java")).unwrap();
    fs::write(fixture.0.join("pom.xml"),"<project><modelVersion>4.0.0</modelVersion><groupId>smoke</groupId><artifactId>demo</artifactId><version>1</version></project>").unwrap();
    fs::write(
        fixture.0.join("src/main/java/Example.java"),
        "public class Example {}\n",
    )
    .unwrap();
    for selection in ["java", "all"] {
        let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
            .args(["check", selection])
            .arg(&fixture.0)
            .args([
                "--format=json",
                "--timeout",
                "2s",
                "--jobs",
                "1",
                "--maven-tool",
            ])
            .arg(fixture.0.join("missing-mvn"))
            .env("PATH", "")
            .env_remove("CODEGUARD_TIMEOUT")
            .env_remove("CODEGUARD_JOBS")
            .env_remove("JAVA_HOME")
            .env_remove("MAVEN_HOME")
            .env_remove("M2_HOME")
            .output()
            .unwrap();
        assert_eq!(
            output.status.code(),
            Some(3),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let raw = &output.stdout;
        let document: serde_json::Value =
            serde_json::from_slice(raw).expect("actual provider JSON");
        assert_eq!(document["report_type"], "check_feedback");
        assert_eq!(document["authority"], "local_unverified");
        assert!(document["required_obligations"].is_null());
        if document["native_results"]["java_p3c"].is_object() {
            assert_eq!(
                document["native_results"]["java_p3c"]["schema_version"],
                "0.3.0"
            );
        }
        let projected = project_raw(raw, None);
        if projected
            .issues()
            .iter()
            .any(|issue| issue.contains("exact closed Schema"))
        {
            // 这里只含本测试新建的受控Java/POM与缺工具路径；绝不在产品入口打印用户原件。
            panic!(
                "controlled CLI fixture selection={selection}; issues={:?}\nCONTROLLED_FEEDBACK_JSON_BEGIN\n{}\nCONTROLLED_FEEDBACK_JSON_END",
                projected.issues(),
                serde_json::to_string_pretty(&document).unwrap()
            );
        }
        assert_eq!(projected.original_bytes(), raw);
        assert_eq!(projected.trust_level(), "UNVERIFIED");
        assert_eq!(projected.status(), "incomplete");
        assert!(
            projected
                .issues()
                .iter()
                .any(|s| s.contains("lacks_frozen_obligation_evidence")),
            "{:?}",
            projected.issues()
        );
        assert!(!matches!(
            projected.gate().decision,
            DeliveryDecision::Allow | DeliveryDecision::AllowWithExceptions
        ));
        assert_eq!(
            document["delivery_decision"],
            if selection == "all" {
                "incomplete"
            } else {
                "not_evaluated"
            }
        );
        assert!(!fixture.0.join(".codegraph").exists());
        assert!(!fixture.0.join("openspec").exists());
    }
}
