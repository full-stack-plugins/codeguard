use codeguard_adapters::parse_gradle_checker_model;
use serde_json::{Value, json};
#[test]
fn official_task_identity_requires_plugin_class_and_enabled_state() {
    for (plugin, implementation, enabled, expected) in [
        (
            true,
            "org.owasp.dependencycheck.gradle.tasks.Analyze",
            true,
            1,
        ),
        (
            true,
            "org.owasp.dependencycheck.gradle.tasks.Aggregate",
            true,
            1,
        ),
        (true, "org.gradle.api.DefaultTask", true, 0),
        (
            true,
            "org.owasp.dependencycheck.gradle.tasks.Analyze_Decorated",
            true,
            0,
        ),
        (
            true,
            "org.owasp.dependencycheck.gradle.tasks.Analyze",
            false,
            0,
        ),
        (
            false,
            "org.owasp.dependencycheck.gradle.tasks.Analyze",
            true,
            0,
        ),
    ] {
        let mut value = model();
        value["projects"][1]["plugins"] = if plugin {
            json!(["org.owasp.dependencycheck"])
        } else {
            json!([])
        };
        value["projects"][1]["tasks"] =
            json!([{"name":"customScan", "implementation":implementation,"enabled":enabled}]);
        let parsed = parse_gradle_checker_model(&serde_json::to_vec(&value).unwrap()).unwrap();
        assert_eq!(parsed.projects[1].dependency_check_tasks().len(), expected);
    }
}

#[test]
fn parser_rejects_path_aliases_unknown_shapes_and_unresolved_versions() {
    for path in [
        "app/../other",
        "app//build",
        "app/./build",
        "C:/build",
        "app\\build",
        ".",
        "",
    ] {
        let mut value = model();
        value["projects"][1]["build_dir"] = json!(path);
        assert!(
            parse_gradle_checker_model(&serde_json::to_vec(&value).unwrap()).is_err(),
            "{path}"
        );
    }
    for version in ["", "unknown", "8", "8.10-rc-1", "8.10.2.1"] {
        let mut value = model();
        value["gradle_version"] = json!(version);
        assert!(
            parse_gradle_checker_model(&serde_json::to_vec(&value).unwrap()).is_err(),
            "{version}"
        );
    }
    let mut value = model();
    value["projects"][1]["tasks"][0]["production_qualified"] = json!(true);
    assert!(parse_gradle_checker_model(&serde_json::to_vec(&value).unwrap()).is_err());
    assert!(parse_gradle_checker_model(&vec![b' '; 1024 * 1024 + 1]).is_err());
}
fn model() -> Value {
    json!({"schema_version":"0.1.0","report_type":"gradle_checker_model","gradle_version":"8.10.2","included_build_count":0,"projects":[{"path":":","project_dir":".","build_dir":"build","plugins":["java"],"tasks":[{"name":"dependencyCheckAnalyze","implementation":"org.gradle.api.DefaultTask","enabled":true}]},{"path":":app","project_dir":"app","build_dir":"app/build","plugins":["java","org.owasp.dependencycheck"],"tasks":[{"name":"dependencyCheckAnalyze","implementation":"org.owasp.dependencycheck.gradle.tasks.Analyze","enabled":true},{"name":"dependencyCheckAggregate","implementation":"org.owasp.dependencycheck.gradle.tasks.Aggregate","enabled":false}]}]})
}
#[test]
fn subprojects_keep_their_own_official_task_types_and_disabled_state() {
    let parsed = parse_gradle_checker_model(&serde_json::to_vec(&model()).unwrap()).unwrap();
    assert!(parsed.projects[0].dependency_check_tasks().is_empty());
    assert_eq!(parsed.projects[1].dependency_check_tasks().len(), 1);
    let mut no_plugin = model();
    no_plugin["projects"][1]["plugins"] = json!(["java"]);
    assert!(
        parse_gradle_checker_model(&serde_json::to_vec(&no_plugin).unwrap())
            .unwrap()
            .projects[1]
            .dependency_check_tasks()
            .is_empty()
    );
}
#[test]
fn malformed_or_incomplete_native_models_cannot_become_capability_claims() {
    for index in 0..10 {
        let mut bad = model();
        match index {
            0 => bad["included_build_count"] = json!(1),
            1 => bad["projects"][1]["build_dir"] = json!("../../outside"),
            2 => bad["projects"][1]["project_dir"] = json!("/outside"),
            3 => bad["projects"][1]["path"] = json!(":missing:app"),
            4 => {
                let duplicate = bad["projects"][0].clone();
                bad["projects"].as_array_mut().unwrap().push(duplicate);
            }
            5 => {
                let duplicate = bad["projects"][1]["tasks"][0].clone();
                bad["projects"][1]["tasks"]
                    .as_array_mut()
                    .unwrap()
                    .push(duplicate);
            }
            6 => bad["schema_version"] = json!("2.0.0"),
            7 => bad["production_qualified"] = json!(true),
            8 => bad["projects"][1]["plugins"] = json!(["java", "java"]),
            _ => {
                bad["projects"].as_array_mut().unwrap().remove(0);
            }
        };
        assert!(
            parse_gradle_checker_model(&serde_json::to_vec(&bad).unwrap()).is_err(),
            "{index}"
        );
    }
    let duplicate = serde_json::to_string(&model()).unwrap().replacen(
        "\"gradle_version\":\"8.10.2\"",
        "\"gradle_version\":\"8.10.2\",\"gradle_version\":\"8.10.2\"",
        1,
    );
    assert!(parse_gradle_checker_model(duplicate.as_bytes()).is_err());
}
