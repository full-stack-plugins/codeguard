use std::collections::BTreeSet;

use crate::{HookEvent, HookTriggerAction, HookTriggerInput, HookTriggerPlan, HookWriteOutcome};

/// 把宿主事件映射为检查阶段，交付事件始终要求重新取得真实范围。
/// 返回候选计划或损坏事件输入；不执行检查、复用缓存或签发门禁结论。
pub fn plan_hook_trigger(input: &HookTriggerInput) -> Result<HookTriggerPlan, &'static str> {
    let mut plan = HookTriggerPlan {
        action: HookTriggerAction::DiscoverProject,
        target_paths: Vec::new(),
        task_id: None,
        requires_git_snapshot: false,
        soft_result_reuse_candidate: false,
        host_blocking_claimed: false,
        may_claim_delivery: false,
    };
    match input.event {
        HookEvent::SessionStart => {}
        HookEvent::PromptSubmitted => plan.action = HookTriggerAction::ShowIntentGuidance,
        HookEvent::FileChanged => {
            let mut seen = BTreeSet::new();
            for path in &input.changed_paths {
                if !valid_relative_path(path) {
                    return Err("hook_changed_path_invalid");
                }
                if seen.insert(path.as_str()) {
                    plan.target_paths.push(path.clone());
                }
            }
            plan.action = match input.write_outcome {
                HookWriteOutcome::Failed => HookTriggerAction::NoCheck,
                HookWriteOutcome::Confirmed if !plan.target_paths.is_empty() => {
                    plan.soft_result_reuse_candidate = true;
                    HookTriggerAction::FastFileCheck
                }
                HookWriteOutcome::Confirmed | HookWriteOutcome::Unknown => {
                    HookTriggerAction::ResolveChangedScope
                }
            };
        }
        HookEvent::RepairReady => {
            let id = input
                .task_id
                .as_deref()
                .filter(|id| valid_task_id(id))
                .ok_or("hook_task_id_invalid")?;
            plan.action = HookTriggerAction::VerifyTask;
            plan.task_id = Some(id.into());
        }
        HookEvent::PreCommit => {
            plan.action = HookTriggerAction::CommitGate;
            plan.requires_git_snapshot = true;
            plan.host_blocking_claimed = input.host_claims_blocking;
        }
        HookEvent::PrePush => {
            plan.action = HookTriggerAction::PushGate;
            plan.requires_git_snapshot = true;
            plan.host_blocking_claimed = input.host_claims_blocking;
        }
        HookEvent::Ci => {
            plan.action = HookTriggerAction::FullProjectCheck;
            plan.host_blocking_claimed = input.host_claims_blocking;
        }
        HookEvent::Stop => plan.action = HookTriggerAction::ShowSummary,
    }
    Ok(plan)
}

fn valid_relative_path(path: &str) -> bool {
    !path.is_empty()
        && !path.starts_with('/')
        && !path.contains('\\')
        && !path.chars().any(char::is_control)
        && path
            .split('/')
            .all(|part| !part.is_empty() && part != "." && part != "..")
}

fn valid_task_id(id: &str) -> bool {
    id.len() > 3
        && id.len() <= 64
        && id.starts_with("CG-")
        && id[3..]
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
}
