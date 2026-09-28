use codeguard_adapters::parse_owasp_dependency_check_json;

const REPORT: &str = r#"{
  "reportSchema":"1.1",
  "scanInfo":{"engineVersion":"12.1.0","dataSource":[{"name":"NVD CVE Checked","timestamp":"2026-09-25T10:00:00"}]},
  "projectInfo":{"name":"demo","reportDate":"2026-09-26T10:00:00Z"},
  "dependencies":[{"isVirtual":false,"fileName":"demo.jar","filePath":"/private/demo.jar",
    "sha256":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
    "packages":[{"id":"pkg:maven/demo/demo@1"}],
    "vulnerabilities":[{"source":"NVD","name":"CVE-2026-1234","cvssv3":{"baseScore":8.1}}],
    "suppressedVulnerabilities":[{"source":"NVD","name":"CVE-2025-9999"}]
  }]
}"#;

#[test]
fn retains_active_and_native_suppressed_advisories_without_leaking_file_path() {
    let report = parse_owasp_dependency_check_json(REPORT.as_bytes()).unwrap();
    assert_eq!(report.engine_version, "12.1.0");
    assert_eq!(report.dependency_count, 1);
    assert_eq!(report.advisories.len(), 2);
    assert_eq!(report.advisories[0].advisory_id, "CVE-2026-1234");
    assert_eq!(report.advisories[0].score, Some(8.1));
    assert!(!report.advisories[0].suppressed_by_native_tool);
    assert_eq!(report.advisories[0].package_ids, ["pkg:maven/demo/demo@1"]);
    assert!(report.advisories[1].suppressed_by_native_tool);
    assert_eq!(report.advisories[1].score, None);
    assert!(!format!("{report:?}").contains("/private/demo.jar"));
}

#[test]
fn missing_dependencies_or_scan_identity_cannot_be_an_empty_clean_report() {
    for broken in [
        REPORT.replace("\"dependencies\":[", "\"notDependencies\":["),
        REPORT.replace("\"reportSchema\":\"1.1\"", "\"reportSchema\":\"2.0\""),
        REPORT.replace("\"engineVersion\":\"12.1.0\"", "\"engineVersion\":\"\""),
        REPORT.replace(
            "\"reportDate\":\"2026-09-26T10:00:00Z\"",
            "\"reportDate\":null",
        ),
        REPORT.replace("\"dataSource\":[", "\"dataSource\":null,\"other\":["),
        REPORT.replace(
            "\"dataSource\":[",
            "\"analysisExceptions\":[{}],\"dataSource\":[",
        ),
        REPORT.replace(
            "\"dependencies\":[",
            "\"error\":\"failed\",\"dependencies\":[",
        ),
        REPORT.replace(
            "\"dependencies\":[",
            "\"dependencies\":[],\"dependencies\":[",
        ),
    ] {
        assert!(
            parse_owasp_dependency_check_json(broken.as_bytes()).is_err(),
            "{broken}"
        );
    }
}

#[test]
fn no_data_sources_is_retained_as_missing_freshness_evidence() {
    let empty_source = REPORT.replace(
        "{\"name\":\"NVD CVE Checked\",\"timestamp\":\"2026-09-25T10:00:00\"}",
        "",
    );
    let parsed = parse_owasp_dependency_check_json(empty_source.as_bytes()).unwrap();
    assert!(parsed.data_sources.is_empty());
    assert_eq!(parsed.advisories.len(), 2);
}

#[test]
fn invalid_advisory_score_or_identity_fails_instead_of_disappearing() {
    for broken in [
        REPORT.replace("\"name\":\"CVE-2026-1234\"", "\"name\":\"\""),
        REPORT.replace("\"baseScore\":8.1", "\"baseScore\":11"),
        REPORT.replace("\"baseScore\":8.1", "\"baseScore\":\"8.1\""),
        REPORT.replace(
            "\"vulnerabilities\":[",
            "\"vulnerabilities\":null,\"other\":[",
        ),
        REPORT.replace(
            "\"sha256\":\"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\"",
            "\"sha256\":\"bad\"",
        ),
    ] {
        assert!(
            parse_owasp_dependency_check_json(broken.as_bytes()).is_err(),
            "{broken}"
        );
    }
}

#[test]
fn empty_report_is_only_structural_observation_and_unscored_advisory_survives() {
    let clean = REPORT.replace(
        "\"vulnerabilities\":[{\"source\":\"NVD\",\"name\":\"CVE-2026-1234\",\"cvssv3\":{\"baseScore\":8.1}}]",
        "\"vulnerabilities\":[]",
    ).replace(
        "\"suppressedVulnerabilities\":[{\"source\":\"NVD\",\"name\":\"CVE-2025-9999\"}]",
        "\"suppressedVulnerabilities\":[]",
    );
    let parsed = parse_owasp_dependency_check_json(clean.as_bytes()).unwrap();
    assert!(parsed.advisories.is_empty());
    assert_eq!(parsed.dependency_count, 1);
    let unscored = REPORT.replace("\"cvssv3\":{\"baseScore\":8.1}", "\"unscored\":\"true\"");
    let parsed = parse_owasp_dependency_check_json(unscored.as_bytes()).unwrap();
    assert_eq!(parsed.advisories[0].score, None);
}
