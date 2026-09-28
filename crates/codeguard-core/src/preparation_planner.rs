//! 由 readiness 的当前结论规划准备动作，不访问文件或工具。
use crate::readiness::{valid_hash, valid_id};
use crate::{
    PreparationAction, PreparationEvidenceState, PreparationPlan, PreparationTask, ReadinessInput,
    evaluate_readiness,
};
use std::collections::{BTreeMap, BTreeSet};

/// 按已核验要求与观察生成稳定准备计划。
/// 参数是宿主冻结的输入；结果不执行动作，不关闭发现或签发质量通过。
#[must_use]
pub fn plan_preparation(input: &ReadinessInput) -> PreparationPlan {
    let readiness = evaluate_readiness(input);
    let mut tasks = Vec::new();
    if input.requirements_verified && input.observations_verified {
        let mut requirements = BTreeMap::new();
        let mut ambiguous = BTreeSet::new();
        for requirement in &input.requirements {
            if requirements
                .insert(requirement.id.as_str(), requirement)
                .is_some()
            {
                ambiguous.insert(requirement.id.as_str());
            }
        }
        let blocked: BTreeSet<_> = readiness.blocked_ids.iter().map(String::as_str).collect();
        let unresolved: BTreeSet<_> = readiness
            .unresolved_ids
            .iter()
            .map(String::as_str)
            .collect();
        for (id, requirement) in requirements {
            if ambiguous.contains(id) || !valid_id(id) || !valid_hash(&requirement.binding_sha256) {
                continue;
            }
            let (action, confirmed_state) = if blocked.contains(id) {
                // blocked 来自同一判定输入，已保证唯一、绑定一致且当前有效。
                let Some(observation) = input.observations.iter().find(|value| value.id == id)
                else {
                    continue;
                };
                let action = match observation.state {
                    PreparationEvidenceState::Missing => {
                        PreparationAction::RestoreMissingPrerequisite
                    }
                    PreparationEvidenceState::Incompatible => {
                        PreparationAction::ResolveCompatibility
                    }
                    PreparationEvidenceState::Conflict => PreparationAction::ResolveConflict,
                    _ => continue,
                };
                (action, Some(observation.state))
            } else if unresolved.contains(id) {
                (PreparationAction::ReverifyPrerequisite, None)
            } else {
                continue;
            };
            let instruction = match action {
                PreparationAction::RestoreMissingPrerequisite => {
                    "核对该前置的批准要求与缺失证据，恢复对应工具或配置；环境任务不得改写无关源码。"
                }
                PreparationAction::ResolveCompatibility => {
                    "比较批准版本要求与本次工具证据，明确兼容方案；不得降低质量规则或将包版本冒充语言版本。"
                }
                PreparationAction::ResolveConflict => {
                    "保留冲突双方的配置来源，提出具体选择及影响；未经决策不能覆盖项目配置。"
                }
                PreparationAction::ReverifyPrerequisite => {
                    "重新核对适用性、绑定与证据来源，并解析或探测该前置；旧诊断不能证明当前缺失。"
                }
            };
            tasks.push(PreparationTask {
                task_key: format!("preparation:{id}"),
                prerequisite_id: id.into(),
                binding_sha256: requirement.binding_sha256.clone(),
                action,
                confirmed_state,
                instruction: instruction.into(),
                close_condition: "宿主重新核验同一前置当前绑定下的原生/配置证据，确认适用且有效满足；任务勾选不能关闭，准备完成不能代替质量检查。".into(),
                automatic_execution_authorized: false,
            });
        }
    }
    PreparationPlan { readiness, tasks }
}
