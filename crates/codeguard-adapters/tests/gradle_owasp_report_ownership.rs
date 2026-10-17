use codeguard_adapters::{parse_gradle_checker_model, parse_gradle_owasp_report_ownership};
use serde_json::{Value, json};
fn model() -> codeguard_adapters::GradleCheckerModel {
    parse_gradle_checker_model(&serde_json::to_vec(&json!({"schema_version":"0.1.0","report_type":"gradle_checker_model","gradle_version":"8.10.2","included_build_count":0,"projects":[{"path":":","project_dir":".","build_dir":"build","plugins":["org.owasp.dependencycheck"],"tasks":[{"name":"dependencyCheckAnalyze","implementation":"org.owasp.dependencycheck.gradle.tasks.Analyze","enabled":true}]}]})).unwrap()).unwrap()
}
fn ownership() -> Value {
    json!({"schema_version":"0.1.0","report_type":"gradle_owasp_report_ownership","tasks":[{"project_path":":","task_path":":dependencyCheckAnalyze","implementation":"org.owasp.dependencycheck.gradle.tasks.Analyze","report_project_name":"root project 'sample'","report_path":"build/reports/dependency-check-report.json"}]})
}
#[test]
fn ownership_is_attributed_to_exact_requested_official_task() {
    assert_eq!(
        parse_gradle_owasp_report_ownership(
            &serde_json::to_vec(&ownership()).unwrap(),
            &model(),
            &[":dependencyCheckAnalyze".into()]
        )
        .unwrap()
        .len(),
        1
    );
    for mutation in 0..8 {
        let mut value = ownership();
        match mutation {
            0 => value["tasks"][0]["report_path"] = "../outside.json".into(),
            1 => value["tasks"][0]["report_path"] = "/absolute.json".into(),
            2 => value["tasks"][0]["task_path"] = ":ordinary".into(),
            3 => value["tasks"][0]["implementation"] = "org.gradle.api.DefaultTask".into(),
            4 => value["tasks"][0]["project_path"] = ":other".into(),
            5 => value["tasks"][0]["report_project_name"] = "".into(),
            6 => {
                value["tasks"].as_array_mut().unwrap().clear();
            }
            _ => {
                let item = value["tasks"][0].clone();
                value["tasks"].as_array_mut().unwrap().push(item);
            }
        }
        assert!(
            parse_gradle_owasp_report_ownership(
                &serde_json::to_vec(&value).unwrap(),
                &model(),
                &[":dependencyCheckAnalyze".into()]
            )
            .is_err()
        );
    }
}
#[test]
fn duplicate_json_fields_and_budget_overruns_are_rejected() {
    assert!(
        parse_gradle_owasp_report_ownership(
            br#"{"schema_version":"0.1.0","schema_version":"0.1.0"}"#,
            &model(),
            &[":dependencyCheckAnalyze".into()]
        )
        .is_err()
    );
    assert!(
        parse_gradle_owasp_report_ownership(
            &vec![b' '; 1024 * 1024 + 1],
            &model(),
            &[":dependencyCheckAnalyze".into()]
        )
        .is_err()
    );
}

#[test]
fn two_official_tasks_cannot_claim_one_output_path() {
    let mut model = model();
    let mut task = model.projects[0].tasks[0].clone();
    task.name = "otherScan".into();
    model.projects[0].tasks.push(task);
    let mut doc = ownership();
    let mut record = doc["tasks"][0].clone();
    record["task_path"] = ":otherScan".into();
    doc["tasks"].as_array_mut().unwrap().push(record);
    let paths = vec![":dependencyCheckAnalyze".into(), ":otherScan".into()];
    assert_eq!(
        parse_gradle_owasp_report_ownership(&serde_json::to_vec(&doc).unwrap(), &model, &paths)
            .unwrap_err(),
        "gradle_owasp_report_scope_invalid"
    );
    doc["tasks"][1]["report_path"] = "build/other/dependency-check-report.json".into();
    assert_eq!(
        parse_gradle_owasp_report_ownership(&serde_json::to_vec(&doc).unwrap(), &model, &paths)
            .unwrap()
            .len(),
        2
    );
}
