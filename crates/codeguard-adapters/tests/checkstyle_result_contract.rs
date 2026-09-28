#![cfg(unix)]

use codeguard_adapters::evaluate_checkstyle_report;

fn xml(severity: &str, count: usize) -> Vec<u8> {
    format!(
        "<checkstyle version=\"10.21.4\"><file name=\"/work/Foo.java\">{}</file></checkstyle>",
        format!(
            "<error line=\"1\" severity=\"{severity}\" message=\"doc\" source=\"publicType\"/>"
        )
        .repeat(count)
    )
    .into_bytes()
}

#[test]
fn exit_zero_keeps_native_warnings_and_info() {
    for severity in ["warning", "info"] {
        let result = evaluate_checkstyle_report(
            &xml(severity, 1),
            "10.21.4",
            &["/work/Foo.java".into()],
            Some(0),
        );
        assert!(result.local_coherent);
        assert_eq!(result.parsed.diagnostics.len(), 1);
    }
}

#[test]
fn error_exit_must_match_native_count_and_scope() {
    for (count, exit) in [(0, 0), (1, 1), (3, 3)] {
        assert!(
            evaluate_checkstyle_report(
                &xml("error", count),
                "10.21.4",
                &["/work/Foo.java".into()],
                Some(exit)
            )
            .local_coherent
        );
    }
    for exit in [None, Some(0), Some(2), Some(-1)] {
        let result = evaluate_checkstyle_report(
            &xml("error", 1),
            "10.21.4",
            &["/work/Foo.java".into()],
            exit,
        );
        assert!(!result.local_coherent);
        assert_eq!(result.parsed.diagnostics.len(), 1);
    }
    for scope in [
        vec![],
        vec!["/work/Other.java".into()],
        vec!["/work/Foo.java".into(), "/work/Foo.java".into()],
    ] {
        assert!(
            !evaluate_checkstyle_report(&xml("error", 1), "10.21.4", &scope, Some(1))
                .local_coherent
        );
    }
    assert!(
        !evaluate_checkstyle_report(
            &xml("error", 0),
            "10.21.4",
            &["/work/Foo.java".into()],
            Some(1)
        )
        .local_coherent
    );
}

#[test]
#[cfg(unix)]
fn unix_exit_wrapping_does_not_erase_256_errors() {
    let result = evaluate_checkstyle_report(
        &xml("error", 256),
        "10.21.4",
        &["/work/Foo.java".into()],
        Some(0),
    );
    assert!(result.local_coherent);
    assert_eq!(result.parsed.diagnostics.len(), 256);
}

#[test]
fn version_or_exception_prevents_coherence_even_when_exit_matches() {
    let future = String::from_utf8(xml("error", 1))
        .unwrap()
        .replace("10.21.4", "11.0");
    let result = evaluate_checkstyle_report(
        future.as_bytes(),
        "11.0",
        &["/work/Foo.java".into()],
        Some(1),
    );
    assert!(!result.local_coherent);
    assert_eq!(result.reason, Some("checkstyle_exit_contract_unverified"));
    assert!(
        !evaluate_checkstyle_report(
            &xml("error", 1),
            "11.0",
            &["/work/Foo.java".into()],
            Some(1)
        )
        .local_coherent
    );
    let bytes=b"<checkstyle version=\"10.21.4\"><file name=\"/work/Foo.java\"><exception>broken</exception></file></checkstyle>";
    let result = evaluate_checkstyle_report(bytes, "10.21.4", &["/work/Foo.java".into()], Some(0));
    assert!(!result.local_coherent);
    assert_eq!(result.parsed.processing_errors, 1);
}
