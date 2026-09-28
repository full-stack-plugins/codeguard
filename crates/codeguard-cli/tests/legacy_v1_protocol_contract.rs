use codeguard_cli::legacy_v1_protocol::{
    LegacyV1Entry as Entry, LegacyV1Signal as Signal, project_legacy_v1,
};

#[test]
fn old_check_cve_and_dockerfile_have_distinct_mixed_priorities() {
    let cases: &[(Entry, &[Signal], u8)] = &[
        (Entry::Check, &[], 1),
        (Entry::Check, &[Signal::Pass, Signal::Skipped], 0),
        (Entry::Check, &[Signal::Pass, Signal::Planned], 1),
        (Entry::Check, &[Signal::Fail, Signal::Unverified], 2),
        (Entry::Cve, &[], 1),
        (Entry::Cve, &[Signal::Pass], 0),
        (Entry::Cve, &[Signal::Fail, Signal::Unverified], 2),
        (Entry::Cve, &[Signal::InvalidInput], 3),
        (Entry::Dockerfile, &[], 1),
        (Entry::Dockerfile, &[Signal::Pass, Signal::Fail], 2),
        (Entry::Dockerfile, &[Signal::Fail, Signal::Unverified], 1),
        (Entry::Dockerfile, &[Signal::Pass, Signal::Unverified], 1),
    ];
    for &(entry, signals, expected) in cases {
        let actual = project_legacy_v1(entry, signals).unwrap();
        assert_eq!(actual.legacy_exit_code, expected, "{entry:?} {signals:?}");
        assert_eq!(actual.new_delivery_decision, "not_evaluated");
    }
}

#[test]
fn management_fix_and_hook_exits_do_not_become_quality_allow() {
    let cases: &[(Entry, &[Signal], u8)] = &[
        (Entry::Fix, &[Signal::NoLanguages], 1),
        (Entry::Fix, &[Signal::NoChangedFiles], 0),
        (Entry::Fix, &[Signal::NoApplicableFiles], 0),
        (Entry::Fix, &[Signal::FormatterSucceeded], 0),
        (
            Entry::Fix,
            &[Signal::FormatterSucceeded, Signal::FormatterFailed],
            1,
        ),
        (Entry::Fix, &[Signal::Skipped, Signal::DryRun], 0),
        (
            Entry::Fix,
            &[Signal::FormatterSucceeded, Signal::Planned],
            1,
        ),
        (Entry::Fix, &[Signal::Skipped, Signal::Unverified], 1),
        (Entry::Detect, &[Signal::Pass], 0),
        (Entry::Detect, &[Signal::Unverified], 1),
        (Entry::Init, &[], 0),
        (Entry::JavaPlan, &[Signal::Planned], 0),
        (Entry::JavaPlan, &[Signal::Skipped], 0),
        (Entry::JavaPlan, &[Signal::Unverified], 1),
        (Entry::UnknownCommand, &[], 1),
        (Entry::PreToolGitGuard, &[Signal::HookAllowed], 0),
        (Entry::PreToolGitGuard, &[Signal::HookBlocked], 2),
        (Entry::ObservingHook, &[Signal::HookAllowed], 0),
    ];
    for &(entry, signals, expected) in cases {
        let actual = project_legacy_v1(entry, signals).unwrap();
        assert_eq!(actual.legacy_exit_code, expected, "{entry:?} {signals:?}");
        assert_eq!(actual.new_delivery_decision, "not_evaluated");
    }
}

#[test]
fn parser_usage_and_unrecognized_signal_combinations_are_not_guessed() {
    for entry in [Entry::Check, Entry::Dockerfile, Entry::Fix, Entry::JavaPlan] {
        let result = project_legacy_v1(entry, &[Signal::InvalidInput]).unwrap();
        assert_eq!(result.legacy_exit_code, 2, "{entry:?}");
        assert_eq!(result.new_delivery_decision, "not_evaluated");
    }
    for (entry, signals) in [
        (Entry::Fix, vec![]),
        (Entry::Init, vec![Signal::Fail]),
        (Entry::PreToolGitGuard, vec![Signal::Fail]),
        (Entry::ObservingHook, vec![Signal::HookBlocked]),
        (Entry::Cve, vec![Signal::Skipped]),
        (Entry::Detect, vec![Signal::Pass, Signal::Pass]),
    ] {
        assert!(project_legacy_v1(entry, &signals).is_none(), "{entry:?}");
    }
}
