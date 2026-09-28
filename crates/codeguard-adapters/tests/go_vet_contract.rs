use codeguard_adapters::{GoVetParseState, parse_go_vet_json};
use std::path::Path;
use std::process::Command;

const ROOT: &str = "/tmp/codeguard-go-vet";

fn report(position: &str) -> String {
    format!(
        "# example.com/demo\n{{\"example.com/demo\":{{\"printf\":[{{\"posn\":\"{position}\",\"message\":\"wrong format\"}}]}}}}\n"
    )
}

#[test]
fn exit_zero_with_native_json_finding_is_not_a_clean_result() {
    let raw = report("/tmp/codeguard-go-vet/main.go:6:5");
    let parsed = parse_go_vet_json(
        Path::new(ROOT),
        "go1.23.4",
        "go1.23.4",
        0,
        b"",
        raw.as_bytes(),
    );
    assert_eq!(
        parsed.state,
        GoVetParseState::FindingsObservedUnverified,
        "{parsed:?}"
    );
    assert_eq!(parsed.findings.len(), 1);
    assert_eq!(parsed.findings[0].native_rule_id, "printf");
    assert_eq!(parsed.findings[0].path, "main.go");
}

#[test]
fn clean_native_object_is_observed_but_does_not_prove_project_coverage() {
    let parsed = parse_go_vet_json(
        Path::new(ROOT),
        "go1.23.4",
        "go1.23.4",
        0,
        b"",
        b"# example.com/demo\n{}\n",
    );
    assert_eq!(parsed.state, GoVetParseState::CleanObservedUnverified);
    assert!(parsed.findings.is_empty());
}

#[test]
fn tool_failure_bad_json_or_wrong_version_never_yields_partial_findings() {
    let good = report("/tmp/codeguard-go-vet/main.go:6:5");
    for (version, exit, stdout, stderr) in [
        ("go1.23.4", 1, b"".as_slice(), good.as_bytes()),
        ("go1.23.5", 0, b"".as_slice(), good.as_bytes()),
        ("go1.23.4", 0, b"noise".as_slice(), good.as_bytes()),
        ("go1.23.4", 0, b"".as_slice(), b"# example.com/demo\n{"),
        (
            "go1.23.4",
            0,
            b"".as_slice(),
            b"# example.com/demo\n./main.go:6:5: compile error",
        ),
    ] {
        let parsed = parse_go_vet_json(Path::new(ROOT), "go1.23.4", version, exit, stdout, stderr);
        assert_eq!(parsed.state, GoVetParseState::Incomplete);
        assert!(parsed.findings.is_empty());
    }
}

#[test]
fn report_cannot_claim_another_package_or_outside_file() {
    for raw in [
        report("/tmp/elsewhere/main.go:6:5"),
        report("/tmp/codeguard-go-vet/../elsewhere/main.go:6:5"),
        "# example.com/other\n{\"example.com/demo\":{\"printf\":[]}}\n".into(),
    ] {
        let parsed = parse_go_vet_json(
            Path::new(ROOT),
            "go1.23.4",
            "go1.23.4",
            0,
            b"",
            raw.as_bytes(),
        );
        assert_eq!(parsed.state, GoVetParseState::Incomplete);
    }
}

#[test]
fn multiple_package_reports_merge_only_when_every_chunk_is_valid() {
    let one = report("/tmp/codeguard-go-vet/a/a.go:3:12");
    let two = "# example.com/other\n{\"example.com/other\":{\"printf\":[{\"posn\":\"/tmp/codeguard-go-vet/b/b.go:4:10\",\"message\":\"wrong format\"}]}}\n";
    let both = format!("{one}{two}");
    let parsed = parse_go_vet_json(
        Path::new(ROOT),
        "go1.23.4",
        "go1.23.4",
        0,
        b"",
        both.as_bytes(),
    );
    assert_eq!(parsed.state, GoVetParseState::FindingsObservedUnverified);
    assert_eq!(parsed.findings.len(), 2);

    let corrupt = format!("{one}# example.com/other\n{{");
    let parsed = parse_go_vet_json(
        Path::new(ROOT),
        "go1.23.4",
        "go1.23.4",
        0,
        b"",
        corrupt.as_bytes(),
    );
    assert_eq!(parsed.state, GoVetParseState::Incomplete);
    assert!(parsed.findings.is_empty());
}

#[test]
#[ignore = "requires explicitly supplied local Go 1.23.4 binary; no installation or network"]
fn native_go_vet_json_distinguishes_finding_clean_and_compile_failure() {
    let go_tool = std::env::var("CODEGUARD_GO_TOOL").expect("指定 CODEGUARD_GO_TOOL");
    let root = std::env::temp_dir().join(format!("codeguard-go-vet-native-{}", std::process::id()));
    std::fs::create_dir_all(&root).unwrap();
    std::fs::write(
        root.join("go.mod"),
        "module example.com/codeguard-vet\n\ngo 1.23\n",
    )
    .unwrap();
    let source = root.join("main.go");
    std::fs::write(
        &source,
        "package main\nimport \"fmt\"\nfunc main() { fmt.Printf(\"%d\", \"bad\") }\n",
    )
    .unwrap();

    let run = || {
        Command::new(&go_tool)
            .args(["vet", "-json", "./..."])
            .current_dir(&root)
            .env("GOPROXY", "off")
            .env("GOSUMDB", "off")
            .env("GOTOOLCHAIN", "local")
            .env("GOWORK", "off")
            .output()
            .unwrap()
    };
    let finding = run();
    let parsed = parse_go_vet_json(
        &root,
        "go1.23.4",
        "go1.23.4",
        finding.status.code().unwrap(),
        &finding.stdout,
        &finding.stderr,
    );
    assert_eq!(
        parsed.state,
        GoVetParseState::FindingsObservedUnverified,
        "{parsed:?}; stderr={}",
        String::from_utf8_lossy(&finding.stderr)
    );
    assert_eq!(parsed.findings.len(), 1);
    assert_eq!(parsed.findings[0].native_rule_id, "printf");

    std::fs::write(
        &source,
        "package main\nimport \"fmt\"\nfunc main() { fmt.Printf(\"%d\", 1) }\n",
    )
    .unwrap();
    let clean = run();
    assert_eq!(
        parse_go_vet_json(
            &root,
            "go1.23.4",
            "go1.23.4",
            clean.status.code().unwrap(),
            &clean.stdout,
            &clean.stderr,
        )
        .state,
        GoVetParseState::CleanObservedUnverified
    );

    std::fs::write(&source, "package main\nfunc main() { missing() }\n").unwrap();
    let broken = run();
    let parsed = parse_go_vet_json(
        &root,
        "go1.23.4",
        "go1.23.4",
        broken.status.code().unwrap(),
        &broken.stdout,
        &broken.stderr,
    );
    assert_eq!(parsed.state, GoVetParseState::Incomplete);
    assert!(parsed.findings.is_empty());
    std::fs::remove_dir_all(root).unwrap();
}
