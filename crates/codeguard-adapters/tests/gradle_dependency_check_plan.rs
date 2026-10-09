use codeguard_adapters::{parse_gradle_checker_model, plan_gradle_dependency_check_tasks};
use serde_json::json;

#[test]
fn explicitly_selected_task_requires_owning_plugin_class_and_enabled_state() {
    for (plugin, implementation, enabled, expected) in [
        (
            true,
            "org.owasp.dependencycheck.gradle.tasks.Analyze",
            true,
            true,
        ),
        (
            true,
            "org.owasp.dependencycheck.gradle.tasks.Aggregate",
            true,
            true,
        ),
        (
            false,
            "org.owasp.dependencycheck.gradle.tasks.Analyze",
            true,
            false,
        ),
        (true, "org.gradle.api.DefaultTask", true, false),
        (
            true,
            "org.owasp.dependencycheck.gradle.tasks.Analyze",
            false,
            false,
        ),
    ] {
        let value = json!({"schema_version":"0.1.0","report_type":"gradle_checker_model","gradle_version":"8.10.2","included_build_count":0,"projects":[{"path":":","project_dir":".","build_dir":"build","plugins":if plugin {vec!["org.owasp.dependencycheck"]}else{vec![]},"tasks":[{"name":"customScan","implementation":implementation,"enabled":enabled}]}]});
        let model = parse_gradle_checker_model(&serde_json::to_vec(&value).unwrap()).unwrap();
        assert_eq!(
            plan_gradle_dependency_check_tasks(&model, &[":customScan".into()]).is_ok(),
            expected
        );
        for requested in [
            vec![],
            vec![":unknown".into()],
            vec![":customScan".into(), ":customScan".into()],
        ] {
            assert!(plan_gradle_dependency_check_tasks(&model, &requested).is_err());
        }
    }
}
