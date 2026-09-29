use codeguard_core::{
    NativeFallbackState, SyntaxPrecheckOutcome, SyntaxPrecheckStatus, SyntaxSetupAction,
    select_syntax_setup_action,
};

fn precheck(status: SyntaxPrecheckStatus) -> SyntaxPrecheckOutcome {
    let selected_files = usize::from(status != SyntaxPrecheckStatus::NotRun);
    SyntaxPrecheckOutcome {
        status,
        scope_complete: status != SyntaxPrecheckStatus::Incomplete,
        cancelled: false,
        selected_files,
        checked_files: usize::from(matches!(
            status,
            SyntaxPrecheckStatus::Clean | SyntaxPrecheckStatus::SuspectedIssue
        )),
        incomplete_files: usize::from(status == SyntaxPrecheckStatus::Incomplete),
        unsupported_files: usize::from(status == SyntaxPrecheckStatus::Unsupported),
        unqualified_files: 0,
        truncated_files: 0,
        suspected_recoveries: usize::from(status == SyntaxPrecheckStatus::SuspectedIssue),
    }
}

#[test]
fn optional_missing_checker_with_complete_clean_precheck_is_only_recommended() {
    assert_eq!(
        select_syntax_setup_action(
            NativeFallbackState::MissingTool,
            &precheck(SyntaxPrecheckStatus::Clean),
            false,
        ),
        SyntaxSetupAction::RecommendNativeTool
    );
}

#[test]
fn required_checker_stays_required_even_when_wasm_is_clean() {
    assert_eq!(
        select_syntax_setup_action(
            NativeFallbackState::MissingTool,
            &precheck(SyntaxPrecheckStatus::Clean),
            true,
        ),
        SyntaxSetupAction::RequireNativeToolAndConfirmation
    );
}

#[test]
fn suspected_or_incomplete_precheck_requires_native_confirmation() {
    for status in [
        SyntaxPrecheckStatus::SuspectedIssue,
        SyntaxPrecheckStatus::Incomplete,
        SyntaxPrecheckStatus::Unsupported,
        SyntaxPrecheckStatus::NotRun,
    ] {
        assert_eq!(
            select_syntax_setup_action(NativeFallbackState::MissingTool, &precheck(status), false),
            SyntaxSetupAction::RequireNativeToolAndConfirmation,
            "status: {status:?}"
        );
    }
}

#[test]
fn invalid_config_and_failed_execution_never_become_install_advice() {
    for status in [
        SyntaxPrecheckStatus::Clean,
        SyntaxPrecheckStatus::SuspectedIssue,
        SyntaxPrecheckStatus::Incomplete,
    ] {
        assert_eq!(
            select_syntax_setup_action(
                NativeFallbackState::InvalidConfiguration,
                &precheck(status),
                false
            ),
            SyntaxSetupAction::RepairNativeConfigurationAndConfirm
        );
        assert_eq!(
            select_syntax_setup_action(
                NativeFallbackState::ExecutionFailed,
                &precheck(status),
                false
            ),
            SyntaxSetupAction::RecoverNativeExecutionAndConfirm
        );
    }
}

#[test]
fn absent_adapter_never_recommends_an_unavailable_checker() {
    for status in [
        SyntaxPrecheckStatus::Clean,
        SyntaxPrecheckStatus::SuspectedIssue,
        SyntaxPrecheckStatus::Incomplete,
    ] {
        assert_eq!(
            select_syntax_setup_action(NativeFallbackState::NoAdapter, &precheck(status), false),
            SyntaxSetupAction::RequireCapabilityDecision
        );
    }
}

#[test]
fn forged_clean_status_without_complete_nonempty_scope_is_not_a_recommendation() {
    let mut claimed = precheck(SyntaxPrecheckStatus::Clean);
    claimed.selected_files = 0;
    assert_eq!(
        select_syntax_setup_action(NativeFallbackState::MissingTool, &claimed, false),
        SyntaxSetupAction::RequireNativeToolAndConfirmation
    );
    claimed = precheck(SyntaxPrecheckStatus::Clean);
    claimed.unqualified_files = 1;
    assert_eq!(
        select_syntax_setup_action(NativeFallbackState::MissingTool, &claimed, false),
        SyntaxSetupAction::RequireNativeToolAndConfirmation
    );
}
