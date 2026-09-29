use crate::{NativeFallbackState, SyntaxPrecheckOutcome, SyntaxPrecheckStatus, SyntaxSetupAction};

/// 选择原生能力缺失时的准备动作。
/// 参数分别为经核对的原生阻塞、完整初检汇总及项目是否已有必需原生义务；
/// 返回供工作台使用的动作，不产生检查结果、批准或交付结论。
pub fn select_syntax_setup_action(
    native_state: NativeFallbackState,
    precheck: &SyntaxPrecheckOutcome,
    native_required: bool,
) -> SyntaxSetupAction {
    match native_state {
        NativeFallbackState::NoAdapter => SyntaxSetupAction::RequireCapabilityDecision,
        NativeFallbackState::InvalidConfiguration => {
            SyntaxSetupAction::RepairNativeConfigurationAndConfirm
        }
        NativeFallbackState::ExecutionFailed => SyntaxSetupAction::RecoverNativeExecutionAndConfirm,
        NativeFallbackState::MissingTool
            if !native_required && complete_clean_precheck(precheck) =>
        {
            SyntaxSetupAction::RecommendNativeTool
        }
        NativeFallbackState::MissingTool => SyntaxSetupAction::RequireNativeToolAndConfirmation,
    }
}

fn complete_clean_precheck(precheck: &SyntaxPrecheckOutcome) -> bool {
    precheck.status == SyntaxPrecheckStatus::Clean
        && precheck.scope_complete
        && !precheck.cancelled
        && precheck.selected_files > 0
        && precheck.checked_files == precheck.selected_files
        && precheck.incomplete_files == 0
        && precheck.unsupported_files == 0
        && precheck.unqualified_files == 0
        && precheck.truncated_files == 0
        && precheck.suspected_recoveries == 0
}
