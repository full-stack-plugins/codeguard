#![cfg(unix)]

use codeguard_adapters::{CheckstyleCommand, evaluate_checkstyle_report};
use codeguard_runtime::{ProcessSpec, Termination, run_process_recorded_with_report};
use std::collections::BTreeMap;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::time::{Duration, Instant};

struct Fixture(PathBuf);

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
#[ignore = "requires explicit Java executable and Checkstyle 10.21.4 all.jar; no installation"]
fn real_checkstyle_comment_rules_keep_custom_identity_and_clean_recheck() {
    let java = PathBuf::from(std::env::var_os("CODEGUARD_JAVA_BIN").expect("显式 Java 路径"));
    let jar =
        PathBuf::from(std::env::var_os("CODEGUARD_CHECKSTYLE_JAR").expect("显式 Checkstyle JAR"));
    assert!(java.is_absolute() && jar.is_absolute());
    let root = std::env::temp_dir()
        .canonicalize()
        .unwrap()
        .join(format!("cg-checkstyle-native-{}", std::process::id()));
    fs::create_dir(&root).unwrap();
    fs::set_permissions(&root, fs::Permissions::from_mode(0o700)).unwrap();
    let fixture = Fixture(root);
    let config = fixture.0.join("checkstyle.xml");
    // 标准 PUBLIC DTD 由 Checkstyle 自包含制品内置解析，不使用外部属性或规则资源。
    fs::write(&config, "<!DOCTYPE module PUBLIC \"-//Checkstyle//DTD Checkstyle Configuration 1.3//EN\" \"https://checkstyle.org/dtds/configuration_1_3.dtd\"><module name=\"Checker\"><module name=\"TreeWalker\"><module name=\"MissingJavadocType\"><property name=\"id\" value=\"publicType\"/></module><module name=\"JavadocMethod\"><property name=\"id\" value=\"publicMethod\"/></module></module></module>").unwrap();
    let source = fixture.0.join("Foo.java");
    let cases = [
        "public class Foo {\n/** Computes a value. */\npublic int get(int value) { return value; }\n}\n",
        "/** Sample API. */\npublic class Foo {\n/** Computes a value.\n * @param value input\n * @return the input\n */\npublic int get(int value) { return value; }\n}\n",
        "public class Foo {\n/** Computes a value. */\npublic int get(int value) { return value; }\n}\n",
    ];
    for (index, text) in cases.into_iter().enumerate() {
        fs::write(&source, text).unwrap();
        if index == 2 {
            let config_text = fs::read_to_string(&config).unwrap();
            fs::write(
                &config,
                config_text.replace(
                    "<module name=\"Checker\">",
                    "<module name=\"Checker\"><property name=\"severity\" value=\"warning\"/>",
                ),
            )
            .unwrap();
        }
        let input = CheckstyleCommand {
            jar: jar.clone(),
            config: config.clone(),
            source: source.clone(),
            report: fixture.0.join(format!("report-{index}.xml")),
        };
        assert!(!input.report.exists());
        let spec = ProcessSpec {
            executable: java.clone(),
            args: input.args().unwrap(),
            cwd: fixture.0.clone(),
            env: BTreeMap::new(),
            stdin: None,
            deadline: Instant::now() + Duration::from_secs(20),
            output_limit_bytes: 1024 * 1024,
        };
        let execution = run_process_recorded_with_report(
            &spec,
            &AtomicBool::new(false),
            &fixture.0,
            &format!("log-{index}"),
            &fixture.0,
            &format!("report-{index}.xml"),
            1024 * 1024,
        )
        .expect("统一 runtime 原生执行及新鲜报告");
        let exit = match execution.outcome.termination {
            Termination::Exited(code) => Some(code),
            _ => None,
        };
        let result = evaluate_checkstyle_report(
            &execution.report,
            "10.21.4",
            &[source.to_string_lossy().into_owned()],
            exit,
        );
        assert!(result.local_coherent, "{result:?}");
        let parsed = result.parsed;
        assert_eq!(parsed.files, [source.to_string_lossy().into_owned()]);
        assert_eq!(parsed.processing_errors, 0);
        if index == 0 {
            assert!(matches!(execution.outcome.termination, Termination::Exited(code) if code > 0));
            assert!(parsed.diagnostics.iter().any(|d| d.source == "publicType"));
            assert!(
                parsed
                    .diagnostics
                    .iter()
                    .any(|d| d.source == "publicMethod")
            );
            let methods: Vec<_> = parsed
                .diagnostics
                .iter()
                .filter(|d| d.source == "publicMethod")
                .collect();
            assert_eq!(methods.len(), 2, "参数与返回文档均须检出");
            assert!(methods.iter().any(|d| d.message.contains("@param")));
            assert!(methods.iter().any(|d| d.message.contains("@return")));
        } else if index == 1 {
            assert_eq!(execution.outcome.termination, Termination::Exited(0));
            assert!(parsed.diagnostics.is_empty());
        } else {
            assert_eq!(execution.outcome.termination, Termination::Exited(0));
            assert_eq!(parsed.diagnostics.len(), 3);
            assert!(parsed.diagnostics.iter().all(|d| d.severity == "warning"));
        }
    }
}
