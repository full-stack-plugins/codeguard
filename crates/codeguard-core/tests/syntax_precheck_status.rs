use codeguard_core::{
    SyntaxFileObservation, SyntaxFileState, SyntaxPrecheckStatus, assess_syntax_precheck,
};

fn file(path: &str, state: SyntaxFileState) -> SyntaxFileObservation {
    SyntaxFileObservation {
        path: path.into(),
        state,
    }
}

fn assess(
    files: &[SyntaxFileObservation],
) -> Result<codeguard_core::SyntaxPrecheckOutcome, &'static str> {
    assess_syntax_precheck(files, true, false)
}

#[test]
fn clean_requires_nonempty_fully_qualified_scope() {
    assert_eq!(assess(&[]).unwrap().status, SyntaxPrecheckStatus::NotRun);
    let clean = assess(&[
        file(
            "src/A.java",
            SyntaxFileState::Checked {
                recoveries: 0,
                grammar_qualified: true,
                truncated: false,
            },
        ),
        file(
            "src/B.java",
            SyntaxFileState::Checked {
                recoveries: 0,
                grammar_qualified: true,
                truncated: false,
            },
        ),
    ])
    .unwrap();
    assert_eq!(clean.status, SyntaxPrecheckStatus::Clean);
    assert_eq!(clean.checked_files, 2);
    assert_eq!(clean.suspected_recoveries, 0);
}

#[test]
fn suspicion_is_never_a_confirmed_native_violation() {
    let result = assess(&[file(
        "src/A.java",
        SyntaxFileState::Checked {
            recoveries: 2,
            grammar_qualified: true,
            truncated: false,
        },
    )])
    .unwrap();
    assert_eq!(result.status, SyntaxPrecheckStatus::SuspectedIssue);
    assert_eq!(result.suspected_recoveries, 2);
}

#[test]
fn partial_scope_keeps_suspicions_but_cannot_be_clean() {
    let result = assess(&[
        file(
            "src/A.java",
            SyntaxFileState::Checked {
                recoveries: 1,
                grammar_qualified: true,
                truncated: false,
            },
        ),
        file(
            "src/B.java",
            SyntaxFileState::Incomplete {
                reason: "timed_out".into(),
            },
        ),
        file(
            "src/C.java",
            SyntaxFileState::Unsupported {
                reason: "embedded_language".into(),
            },
        ),
    ])
    .unwrap();
    assert_eq!(result.status, SyntaxPrecheckStatus::Incomplete);
    assert_eq!(result.suspected_recoveries, 1);
    assert_eq!(result.incomplete_files, 1);
    assert_eq!(result.unsupported_files, 1);
}

#[test]
fn unknown_version_is_not_clean_even_when_tree_has_no_recovery() {
    let result = assess(&[file(
        "src/A.java",
        SyntaxFileState::Checked {
            recoveries: 0,
            grammar_qualified: false,
            truncated: false,
        },
    )])
    .unwrap();
    assert_eq!(result.status, SyntaxPrecheckStatus::Incomplete);
    assert_eq!(result.unqualified_files, 1);
}

#[test]
fn truncated_recovery_scan_keeps_observed_nodes_without_claiming_clean() {
    let result = assess(&[file(
        "src/A.java",
        SyntaxFileState::Checked {
            recoveries: 1,
            grammar_qualified: true,
            truncated: true,
        },
    )])
    .unwrap();
    assert_eq!(result.status, SyntaxPrecheckStatus::Incomplete);
    assert_eq!(result.suspected_recoveries, 1);
    assert_eq!(result.unqualified_files, 0);
    assert_eq!(result.truncated_files, 1);
}

#[test]
fn incomplete_enumeration_and_cancellation_cannot_be_clean() {
    let clean_file = [file(
        "src/A.java",
        SyntaxFileState::Checked {
            recoveries: 0,
            grammar_qualified: true,
            truncated: false,
        },
    )];
    let partial = assess_syntax_precheck(&clean_file, false, false).unwrap();
    assert_eq!(partial.status, SyntaxPrecheckStatus::Incomplete);
    assert!(!partial.scope_complete);
    let cancelled = assess_syntax_precheck(&clean_file, true, true).unwrap();
    assert_eq!(cancelled.status, SyntaxPrecheckStatus::Incomplete);
    assert!(cancelled.cancelled);
    assert_eq!(
        assess_syntax_precheck(&[], false, false).unwrap().status,
        SyntaxPrecheckStatus::Incomplete
    );
}

#[test]
fn unsupported_only_has_distinct_status_and_duplicate_paths_are_rejected() {
    let unsupported = assess(&[file(
        "src/A.java",
        SyntaxFileState::Unsupported {
            reason: "dialect".into(),
        },
    )])
    .unwrap();
    assert_eq!(unsupported.status, SyntaxPrecheckStatus::Unsupported);
    assert!(
        assess(&[
            file(
                "src/A.java",
                SyntaxFileState::Checked {
                    recoveries: 0,
                    grammar_qualified: true,
                    truncated: false
                }
            ),
            file(
                "src/A.java",
                SyntaxFileState::Checked {
                    recoveries: 0,
                    grammar_qualified: true,
                    truncated: false
                }
            ),
        ])
        .is_err()
    );
    assert!(
        assess(&[file(
            "../outside.java",
            SyntaxFileState::Unsupported {
                reason: "dialect".into()
            }
        ),])
        .is_err()
    );
    assert!(
        assess(&[file(
            "src/A.java",
            SyntaxFileState::Incomplete {
                reason: String::new(),
            },
        )])
        .is_err()
    );
    assert!(
        assess(&[file(
            "src/A.java",
            SyntaxFileState::Unsupported {
                reason: String::new(),
            },
        )])
        .is_err()
    );
}
