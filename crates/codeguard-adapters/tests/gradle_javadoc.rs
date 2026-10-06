use std::collections::BTreeMap;
use codeguard_adapters::{
    parse_gradle_checker_model, plan_gradle_javadoc_tasks, parse_gradle_javadoc_output,
};
use serde_json::json;

#[test]
fn only_enabled_official_javadoc_tasks_in_java_projects_become_fully_qualified_paths() {
    let value = json!({"schema_version":"0.1.0","report_type":"gradle_checker_model","gradle_version":"8.10.2","included_build_count":0,"projects":[{"path":":","project_dir":".","build_dir":"build","plugins":["java"],"tasks":[{"name":"javadoc","implementation":"org.gradle.api.DefaultTask","enabled":true}]},{"path":":app","project_dir":"app","build_dir":"app/build","plugins":["java-library"],"tasks":[{"name":"customDocs","implementation":"org.gradle.api.tasks.javadoc.Javadoc","enabled":true},{"name":"javadoc","implementation":"org.gradle.api.tasks.javadoc.Javadoc","enabled":false}]}]});
    let model = parse_gradle_checker_model(&serde_json::to_vec(&value).unwrap()).unwrap();
    let plans = plan_gradle_javadoc_tasks(&model).unwrap();
    assert_eq!(plans.len(), 1);
    assert_eq!(plans[0].task_path, ":app:customDocs");
    let mut bad = model;
    bad.projects[1].path = ":missing:app".into();
    assert!(plan_gradle_javadoc_tasks(&bad).is_err());
}

#[test]
fn native_warning_blocks_preserve_source_identity_and_never_accept_partial_output() {
    let sources = BTreeMap::from([
        (
            "/private/project/A.java".into(),
            b"public class A {}\n".to_vec(),
        ),
        (
            "/private/project/B.java".into(),
            b"public class B {}\n".to_vec(),
        ),
    ]);
    let log=b"/private/project/A.java:1: warning: no comment\npublic class A {}\n       ^\n1 warning\n/private/project/B.java:1: warning: no comment\npublic class B {}\n       ^\n1 warning\n";
    let parsed = parse_gradle_javadoc_output(log, &sources).unwrap();
    assert_eq!(parsed.len(), 2);
    assert_eq!(parsed[0].path, "/private/project/A.java");
    assert_eq!(parsed[1].rule_id, "JavadocMissingComment");
    for bad in [
        String::from_utf8(log.to_vec())
            .unwrap()
            .replace("A.java:1", "Outside.java:1"),
        String::from_utf8(log.to_vec())
            .unwrap()
            .replace("public class A {}", "public class C {}"),
        String::from_utf8(log.to_vec())
            .unwrap()
            .replace("1 warning", "2 warnings"),
        String::from_utf8(log.to_vec()).unwrap() + "unrecognized message\n",
    ] {
        assert!(parse_gradle_javadoc_output(bad.as_bytes(), &sources).is_err());
    }
    assert!(
        parse_gradle_javadoc_output(b"", &sources)
            .unwrap()
            .is_empty()
    );
}

#[test]
fn native_empty_tag_descriptions_are_not_treated_as_documented_code() {
    let sources = BTreeMap::from([(
        "/project/A.java".into(),
        b" * @param value\n * @return\n".to_vec(),
    )]);
    let parsed=parse_gradle_javadoc_output(b"/project/A.java:1: warning: no description for @param\n * @param value\n ^\n/project/A.java:2: warning: no description for @return\n * @return\n ^\n2 warnings\n",&sources).unwrap();
    assert_eq!(parsed[0].rule_id, "JavadocEmptyParamDescription");
    assert_eq!(parsed[1].rule_id, "JavadocEmptyReturnDescription");
}

#[test]
fn empty_comments_main_descriptions_and_throws_keep_distinct_native_rules() {
    let sources = BTreeMap::from([(
        "/project/A.java".into(),
        b"public class A {}\n * @throws IllegalArgumentException\n/** @param value input\n"
            .to_vec(),
    )]);
    let log = b"/project/A.java:1: warning: empty comment\npublic class A {}\n       ^\n/project/A.java:2: warning: no description for @throws\n * @throws IllegalArgumentException\n   ^\n/project/A.java:3: warning: no main description\n/** @param value input\n    ^\n3 warnings\n";
    let parsed = parse_gradle_javadoc_output(log, &sources)
        .expect("observed JDK21 description warnings must remain actionable native facts");
    assert_eq!(
        parsed.iter().map(|d| d.rule_id).collect::<Vec<_>>(),
        [
            "JavadocEmptyComment",
            "JavadocEmptyThrowsDescription",
            "JavadocMissingMainDescription"
        ]
    );
    for bad in [
        String::from_utf8(log.to_vec())
            .unwrap()
            .replace("empty comment", "possibly empty comment"),
        String::from_utf8(log.to_vec())
            .unwrap()
            .replace("public class A {}", "public class B {}"),
        String::from_utf8(log.to_vec())
            .unwrap()
            .replace("3 warnings", "2 warnings"),
    ] {
        assert!(parse_gradle_javadoc_output(bad.as_bytes(), &sources).is_err());
    }
}
