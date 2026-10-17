#![cfg(unix)]
use codeguard_cli::gradle_javadoc_workbench::project;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    fs,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};
static NEXT: AtomicU64 = AtomicU64::new(0);
struct Inputs(PathBuf);
impl Drop for Inputs {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
fn fixture() -> (Inputs, Value, Value) {
    let dir = std::env::temp_dir().canonicalize().unwrap().join(format!(
        "cg-gradle-projector-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir_all(dir.join("src/main/java")).unwrap();
    let files = [
        ("build.gradle", "plugins { id 'java' }\n"),
        ("settings.gradle", "rootProject.name='sample'\n"),
        ("src/main/java/Sample.java", "public class Sample {}\n"),
    ];
    let inputs = Value::Array(
        files
            .iter()
            .map(|(p, b)| {
                fs::write(dir.join(p), b).unwrap();
                json!({"path":p,"sha256":format!("{:x}",Sha256::digest(b.as_bytes()))})
            })
            .collect(),
    );
    let reports: Value = serde_json::from_str(include_str!(
        "../../../tests/acceptance/evidence/gradle-native-javadoc-reports-2026-10-06.json"
    ))
    .unwrap();
    let mut native = reports[0]["report"].clone();
    native["source_snapshot_sha256"] = json!(format!(
        "{:x}",
        Sha256::digest(serde_json::to_vec(&inputs).unwrap())
    ));
    native["findings"] = json!([{"path":"src/main/java/Sample.java","rule_id":"JavadocMissingComment","line":1,"column":8}]);
    (Inputs(dir), inputs, native)
}
#[test]
fn bound_findings_are_distinct_from_stable_environment_and_coverage_observations() {
    let (dir, inputs, native) = fixture();
    let (a, b) = project(&dir.0, &inputs, &native).unwrap();
    assert_eq!(a.len(), 1);
    assert_eq!(a[0]["checker_id"], "java.gradle.javadoc");
    assert_eq!(b.len(), 1);
    let (again, _) = project(&dir.0, &inputs, &native).unwrap();
    assert_eq!(a, again);
    let mut failed = native.clone();
    failed["native_status"] = json!("incomplete");
    failed["reason"] = json!("native_execution_incomplete");
    failed["findings"] = json!([]);
    let (findings, blockers) = project(&dir.0, &inputs, &failed).unwrap();
    assert!(findings.is_empty());
    assert_eq!(blockers.len(), 1);
    let mut other = failed.clone();
    other["reason"] = json!("request_cancelled");
    assert_eq!(
        blockers[0]["id"],
        project(&dir.0, &inputs, &other).unwrap().1[0]["id"]
    );
}
#[test]
fn changed_sources_scope_duplicates_and_forged_rules_cannot_import() {
    let (dir, inputs, native) = fixture();
    for (field, value) in [
        ("coverage_proven", json!(true)),
        ("rule_configuration_complete", json!(true)),
        ("delivery_decision", json!("allow")),
    ] {
        let mut bad = native.clone();
        bad[field] = value;
        assert!(project(&dir.0, &inputs, &bad).is_err());
    }
    let mut bad = native.clone();
    bad["findings"][0]["path"] = json!("../outside.java");
    assert!(project(&dir.0, &inputs, &bad).is_err());
    let mut bad = native.clone();
    bad["findings"][0]["rule_id"] = json!("FakeRule");
    assert!(project(&dir.0, &inputs, &bad).is_err());
    let mut bad = native.clone();
    bad["findings"]
        .as_array_mut()
        .unwrap()
        .push(native["findings"][0].clone());
    assert_eq!(project(&dir.0, &inputs, &bad).unwrap().0.len(), 1);
    fs::write(
        dir.0.join("src/main/java/Sample.java"),
        "public class Changed {}\n",
    )
    .unwrap();
    assert!(project(&dir.0, &inputs, &native).is_err());
}

#[test]
fn incomplete_or_out_of_scope_bindings_and_locations_are_rejected() {
    let (dir, inputs, native) = fixture();
    for (field, value) in [
        ("source_snapshot_sha256", json!("0".repeat(64))),
        ("selected_java_file_count", json!(2)),
        ("unexpected_field", json!(true)),
        ("task_paths", json!([":javadoc", ":javadoc"])),
        ("native_status", json!("empty_output_unverified")),
    ] {
        let mut bad = native.clone();
        bad[field] = value;
        assert!(project(&dir.0, &inputs, &bad).is_err(), "{field}");
    }
    for (field, value) in [
        ("path", json!("src/main/java/Other.java")),
        ("line", json!(999)),
        ("column", json!(999)),
    ] {
        let mut bad = native.clone();
        bad["findings"][0][field] = value;
        assert!(project(&dir.0, &inputs, &bad).is_err(), "{field}");
    }
    let mut reordered = inputs.clone();
    reordered.as_array_mut().unwrap().swap(0, 1);
    assert!(project(&dir.0, &reordered, &native).is_err());
    let mut duplicate = inputs.clone();
    duplicate.as_array_mut().unwrap().push(inputs[0].clone());
    assert!(project(&dir.0, &duplicate, &native).is_err());
}

#[test]
fn moving_the_same_source_anchor_preserves_identity_without_reusing_old_digests() {
    let (dir, mut inputs, mut native) = fixture();
    let original = project(&dir.0, &inputs, &native).unwrap().0;
    let changed = "\npublic class Sample {}\n";
    fs::write(dir.0.join("src/main/java/Sample.java"), changed).unwrap();
    assert!(project(&dir.0, &inputs, &native).is_err());
    inputs[2]["sha256"] = json!(format!("{:x}", Sha256::digest(changed.as_bytes())));
    native["source_snapshot_sha256"] = json!(format!(
        "{:x}",
        Sha256::digest(serde_json::to_vec(&inputs).unwrap())
    ));
    native["findings"][0]["line"] = json!(2);
    let current = project(&dir.0, &inputs, &native).unwrap().0;
    assert_eq!(original[0]["finding_id"], current[0]["finding_id"]);
    assert_ne!(original[0]["source_sha256"], current[0]["source_sha256"]);
    assert_eq!(current[0]["line"], 2);
}
#[test]
#[ignore = "requires explicit existing Gradle8.10.2/JDK21; never installs"]
fn actual_gradle_empty_descriptions_project_with_original_snapshot_binding() {
    use codeguard_cli::gradle_model_probe::Request;
    use codeguard_runtime::SourceSnapshot;
    use std::{
        collections::BTreeSet,
        sync::atomic::AtomicBool,
        time::{Duration, Instant},
    };
    let (dir, _, _) = fixture();
    fs::write(dir.0.join("build.gradle"),"plugins { id 'java' }\ntasks.named('javadoc') { options.addBooleanOption('Xdoclint:missing',true); options.addBooleanOption('quiet',true) }\n").unwrap();
    fs::write(dir.0.join("src/main/java/Sample.java"),"/** Example. */\npublic class Sample {\n    /** Creates the example. */\n    public Sample() {}\n    /** Adds one.\n     * @param value\n     * @return\n     */\n    public int add(int value) { return value + 1; }\n}\n").unwrap();
    let paths = BTreeSet::from([
        PathBuf::from("build.gradle"),
        PathBuf::from("settings.gradle"),
        PathBuf::from("src/main/java/Sample.java"),
    ]);
    let snapshot =
        SourceSnapshot::capture(&dir.0, paths.clone(), 128, 1024 * 1024, 16 * 1024 * 1024).unwrap();
    let inputs = json!(
        snapshot
            .files()
            .iter()
            .map(|(p, b)| json!({"path":p,"sha256":format!("{:x}",Sha256::digest(b))}))
            .collect::<Vec<_>>()
    );
    let request = Request {
        project_root: dir.0.clone(),
        project_files: paths,
        gradle_bundle: std::env::var_os("CODEGUARD_TEST_GRADLE_BUNDLE")
            .expect("existing Gradle")
            .into(),
        java_home: std::env::var_os("CODEGUARD_TEST_JAVA_HOME")
            .expect("existing JDK")
            .into(),
        deadline: Instant::now() + Duration::from_secs(90),
    };
    let native = codeguard_cli::gradle_javadoc_probe::observe(&request, &AtomicBool::new(false));
    assert_eq!(
        native["native_status"], "findings_observed_unverified",
        "{native}"
    );
    let (findings, blockers) = project(&dir.0, &inputs, &native).unwrap();
    assert_eq!(findings.len(), 2);
    assert_eq!(findings[0]["rule_id"], "JavadocEmptyParamDescription");
    assert_eq!(findings[1]["rule_id"], "JavadocEmptyReturnDescription");
    assert_eq!(
        blockers[0]["reason"],
        "gradle_documentation_rules_and_coverage_unverified"
    );
    if let Some(path) = std::env::var_os("CODEGUARD_TEST_GRADLE_PROJECTION_REPORT") {
        fs::write(path,serde_json::to_vec_pretty(&json!({"evidence_kind":"actual_native_projection","inputs":inputs,"native":native,"findings":findings,"blockers":blockers,"coverage_proven":false,"delivery_decision":"not_evaluated"})).unwrap()).unwrap();
    }
}

#[test]
fn description_rules_require_the_new_native_protocol_without_widening_old_reports() {
    let (dir, inputs, native) = fixture();
    for rule in [
        "JavadocEmptyComment",
        "JavadocMissingMainDescription",
        "JavadocEmptyThrowsDescription",
    ] {
        let mut current = native.clone();
        current["findings"][0]["rule_id"] = json!(rule);
        assert!(project(&dir.0, &inputs, &current).is_err());
        current["schema_version"] = json!("0.2.0");
        let findings = project(&dir.0, &inputs, &current).unwrap().0;
        assert_eq!(findings[0]["rule_id"], rule);
        current["schema_version"] = json!("0.3.0");
        assert!(project(&dir.0, &inputs, &current).is_err());
    }
}
