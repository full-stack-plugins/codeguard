use super::{ProjectionCheckerPin, ProjectionRun};
use guard_protocol::{
    EvidenceRef, ProducerIdentity, RuleDefinition, TaskBinding, VerificationPlan,
};
/// 控制端从既有冻结计划独立构造的映射上下文；无Deserialize，不授予可信身份。无Java对应类型。
#[derive(Clone, Debug)]
pub struct ProjectionContext {
    pub(super) plan: VerificationPlan,
    pub(super) task: TaskBinding,
    pub(super) rules: Vec<RuleDefinition>,
    pub(super) producer: ProducerIdentity,
    pub(super) run: ProjectionRun,
    pub(super) raw_evidence: EvidenceRef,
    pub(super) checker_pins: Vec<ProjectionCheckerPin>,
    pub(super) java_executions: Vec<super::projection_java_execution::ProjectionJavaExecution>,
}
impl ProjectionContext {
    /// 绑定计划/任务/批准规则及真实运行元数据；缺数据返回错误，不填当前时间。
    /// raw_evidence须由宿主私有存储保存本次原字节后提供，适配器复核摘要和大小。
    pub fn new(
        plan: &VerificationPlan,
        task: &TaskBinding,
        rules: &[RuleDefinition],
        producer: &ProducerIdentity,
        run: &ProjectionRun,
        raw_evidence: &EvidenceRef,
        checker_pins: &[ProjectionCheckerPin],
    ) -> Result<Self, String> {
        guard_protocol::encode(&guard_protocol::ProtocolDocument::VerificationPlan(
            plan.clone(),
        ))?;
        guard_protocol::encode(&guard_protocol::ProtocolDocument::TaskBinding(task.clone()))?;
        guard_protocol::encode(&guard_protocol::ProtocolDocument::ProducerIdentity(
            producer.clone(),
        ))?;
        if task.contract_digest != plan.contract_digest
            || !task.scope.actions.contains(&plan.action)
        {
            return Err("task/plan binding mismatch".into());
        }
        let mut ids = std::collections::BTreeSet::new();
        for r in rules {
            guard_protocol::encode(&guard_protocol::ProtocolDocument::RuleDefinition(r.clone()))?;
            if !ids.insert(&r.rule_id) {
                return Err("duplicate approved rule".into());
            }
        }
        if task.scope.repository_ids.len() != 1 {
            return Err("legacy report profile supports exactly one repository".into());
        }
        let mut pin_ids = std::collections::BTreeSet::new();
        for pin in checker_pins {
            let native_ids = pin
                .native_rules
                .iter()
                .map(|r| r.native_rule_id.as_str())
                .collect::<std::collections::BTreeSet<_>>();
            if native_ids.len() != pin.native_rules.len() {
                return Err("duplicate approved native rule mapping".into());
            }
            if let Some(path) = &pin.feedback_target {
                if !plan.obligations.iter().any(|o| {
                    o.obligation_id.as_str() == pin.binding.obligation_id
                        && o.targets.iter().any(|t| &t.path == path)
                }) {
                    return Err("feedback source target outside frozen obligation".into());
                }
            }

            if !pin_ids.insert((&pin.binding.obligation_id, &pin.binding.checker_id))
                || !codeguard_core::CHECK_CATEGORIES.contains(&pin.binding.category.as_str())
            {
                return Err("duplicate or invalid native checker mapping".into());
            }
        }
        for o in &plan.obligations {
            let matches = checker_pins
                .iter()
                .filter(|pin| {
                    pin.verifier_id == o.verifier_id
                        && pin.binding.obligation_id == o.obligation_id.as_str()
                })
                .count();
            if matches != 1 {
                return Err("exact approved native checker mapping missing/ambiguous".into());
            }
            for target in &o.targets {
                if target.selector.is_some() {
                    return Err("legacy path coverage does not prove selector coverage".into());
                }
                let path = target.path.to_bytes();
                if !task.scope.repository_ids.contains(&target.repository_id)
                    || !task.scope.paths.iter().any(|root| {
                        let root = root.to_bytes();
                        path == root || path.starts_with(&[root.as_slice(), b"/"].concat())
                    })
                {
                    return Err("plan target outside task approved scope".into());
                }
            }
            let rule = rules
                .iter()
                .find(|r| r.rule_id == o.rule_id)
                .ok_or("missing approved rule")?;
            if rule.owner != guard_protocol::DomainOwner::Codeguard
                || rule.verifier_id != o.verifier_id
            {
                return Err("wrong professional owner or verifier".into());
            }
        }
        if run.execution.status != guard_protocol::ExecutionStatus::NotRun
            && (run.execution.started_at.is_none() || run.execution.finished_at.is_none())
        {
            return Err("actual execution timestamps missing".into());
        }
        if run.execution.status == guard_protocol::ExecutionStatus::Completed
            && (run.execution.started_at.is_none()
                || run.execution.finished_at.is_none()
                || run.execution.exit_code.is_none())
        {
            return Err("actual execution timestamps/exit missing".into());
        }
        if raw_evidence.classification != guard_protocol::Classification::Private {
            return Err("raw report must remain private".into());
        }
        Ok(Self {
            plan: plan.clone(),
            task: task.clone(),
            rules: rules.to_vec(),
            producer: producer.clone(),
            run: run.clone(),
            raw_evidence: raw_evidence.clone(),
            checker_pins: checker_pins.to_vec(),
            java_executions: Vec::new(),
        })
    }
}

impl ProjectionContext {
    /// 绑定宿主本次原生输出和独立源码证据；不同run/subject/目标、重复checker一律拒绝。
    /// parser_version为既有解析器要求的固定插件/PMD版本；不等于工具身份认证。
    pub fn with_java_execution(
        mut self,
        checker_id: guard_protocol::Identifier,
        log: &[u8],
        evidence: &EvidenceRef,
        native_exit: i32,
        parser_version: guard_protocol::Identifier,
        sources: Vec<super::ProjectionSourceProof>,
    ) -> Result<Self, String> {
        super::projection_source_proof::validate_evidence(log, evidence)?;
        if self.run.execution.status == guard_protocol::ExecutionStatus::NotRun {
            return Err("undispatched run cannot carry native Java execution evidence".into());
        }
        let pins = self
            .checker_pins
            .iter()
            .filter(|p| p.binding.checker_id == checker_id.as_str())
            .collect::<Vec<_>>();
        if pins.is_empty() {
            return Err("Java checker not frozen".into());
        }
        if !matches!(checker_id.as_str(), "java.maven.javadoc" | "java.maven.p3c")
            || self
                .java_executions
                .iter()
                .any(|e| e.checker_id == checker_id)
        {
            return Err("unsupported/duplicate Java execution proof".into());
        }
        let targets = self
            .plan
            .obligations
            .iter()
            .filter(|o| {
                pins.iter()
                    .any(|pin| pin.binding.obligation_id == o.obligation_id.as_str())
            })
            .flat_map(|o| &o.targets)
            .collect::<Vec<_>>();
        let mut paths = std::collections::BTreeSet::new();
        let mut native_paths = std::collections::BTreeSet::new();
        for source in &sources {
            if source.run_id != self.run.run_id
                || source.subject_digest != self.plan.subject_digest
                || !targets.contains(&&source.target)
                || !paths.insert(source.target.path.to_bytes())
                || !native_paths.insert(source.native_path.clone())
            {
                return Err("source proof run/subject/target mismatch or duplicate".into());
            }
        }
        self.java_executions
            .push(super::projection_java_execution::ProjectionJavaExecution {
                checker_id,
                log: log.to_vec(),
                evidence: evidence.clone(),
                native_exit,
                parser_version,
                sources,
            });
        Ok(self)
    }
}
