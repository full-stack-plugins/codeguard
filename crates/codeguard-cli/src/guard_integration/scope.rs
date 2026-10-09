use super::reader::NativeEvidence;
use std::collections::{BTreeMap, BTreeSet};
/// Required targets come from the caller's protected policy, never the native report.
pub struct FrozenObligations {
    targets: BTreeMap<String, Vec<String>>,
    requirements: BTreeMap<String, RequiredScope>,
}
/// Protected caller requirements, independently frozen before reading evidence.
pub struct RequiredScope {
    pub targets: Vec<String>,
    pub rules: Vec<String>,
    pub tools: Vec<RequiredTool>,
    pub configurations: Vec<RequiredConfiguration>,
}
pub struct RequiredTool {
    pub id: String,
    pub version: String,
    pub sha256: String,
}
pub struct RequiredConfiguration {
    pub reference: String,
    pub sha256: String,
}
impl FrozenObligations {
    /// The old target-only constructor remains unqualified. This constructor
    /// additionally requires exact rule, tool and configuration dimensions.
    pub fn with_requirements(
        requirements: BTreeMap<String, RequiredScope>,
    ) -> Result<Self, &'static str> {
        super::scope_budget::validate_requirements(&requirements)?;
        fn valid_digest(value: &str) -> bool {
            value.len() == 64
                && value
                    .bytes()
                    .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        }
        fn unique<'a>(mut items: impl Iterator<Item = &'a str>) -> bool {
            let mut seen = BTreeSet::new();
            items.all(|item| !item.trim().is_empty() && seen.insert(item))
        }
        if requirements.is_empty()
            || requirements.iter().any(|(id, scope)| {
                id.trim().is_empty()
                    || scope.targets.is_empty()
                    || scope.rules.is_empty()
                    || scope.tools.is_empty()
                    || scope.configurations.is_empty()
                    || !unique(scope.targets.iter().map(String::as_str))
                    || !unique(scope.rules.iter().map(String::as_str))
                    || !unique(scope.tools.iter().map(|tool| tool.id.as_str()))
                    || !unique(
                        scope
                            .configurations
                            .iter()
                            .map(|config| config.reference.as_str()),
                    )
                    || scope
                        .tools
                        .iter()
                        .any(|tool| tool.version.trim().is_empty() || !valid_digest(&tool.sha256))
                    || scope
                        .configurations
                        .iter()
                        .any(|config| !valid_digest(&config.sha256))
            })
        {
            return Err("required scope dimensions must be nonempty unique exact identities");
        }
        let targets = requirements
            .iter()
            .map(|(id, scope)| (id.clone(), scope.targets.clone()))
            .collect();
        Ok(Self {
            targets,
            requirements,
        })
    }
    pub(crate) fn frozen_targets(&self) -> BTreeMap<String, Vec<String>> {
        let mut targets = self.targets.clone();
        for items in targets.values_mut() {
            items.sort();
        }
        targets
    }
    pub(crate) fn scope_ids(&self) -> Vec<String> {
        self.targets.keys().cloned().collect()
    }
    pub fn new(targets: BTreeMap<String, Vec<String>>) -> Result<Self, &'static str> {
        super::scope_budget::validate(&targets)?;
        if targets.is_empty()
            || targets.iter().any(|(id, items)| {
                id.trim().is_empty()
                    || items.is_empty()
                    || items.iter().any(|item| item.trim().is_empty())
                    || items.iter().collect::<BTreeSet<_>>().len() != items.len()
            })
        {
            return Err("required obligations must be nonempty and unique");
        }
        Ok(Self {
            targets,
            requirements: BTreeMap::new(),
        })
    }
}
/// Complete is intentionally unavailable: no native profile has been qualified.
#[derive(Debug, PartialEq, Eq)]
pub enum ScopeAssessment {
    Partial { gaps: Vec<String> },
}
pub fn assess_scope(evidence: &NativeEvidence, obligations: &FrozenObligations) -> ScopeAssessment {
    let mut gaps = BTreeSet::from(["native profile unqualified".to_owned()]);
    let results = evidence.report().document()["results"]
        .as_array()
        .expect("validated native results");
    for (id, targets) in &obligations.targets {
        let result = results
            .iter()
            .find(|result| result["obligation_id"].as_str() == Some(id));
        for target in targets {
            let observed = result.is_some_and(|result| {
                result["coverage"]["observed"]
                    .as_array()
                    .is_some_and(|items| items.iter().any(|item| item.as_str() == Some(target)))
            });
            if !observed {
                gaps.insert(format!("missing target {id}:{target}"));
            }
        }
        if !result.is_some_and(|result| result["completion"] == "complete") {
            gaps.insert(format!("incomplete obligation {id}"));
        }
    }
    for (id, scope) in &obligations.requirements {
        // run_report1.0 has global tool identities but no executed-rule or
        // per-obligation configuration observations. Findings and policy hashes
        // cannot substitute for these missing dimensions.
        for rule in &scope.rules {
            gaps.insert(format!("missing native rule coverage {id}:{rule}"));
        }
        for config in &scope.configurations {
            gaps.insert(format!(
                "missing native configuration coverage {id}:{}",
                config.reference
            ));
        }
        for tool in &scope.tools {
            let matches = evidence.report().document()["identities"]["tools"]
                .as_array()
                .is_some_and(|tools| {
                    tools.iter().any(|observed| {
                        observed["id"].as_str() == Some(tool.id.as_str())
                            && observed["version"].as_str() == Some(tool.version.as_str())
                            && observed["binary_sha256"].as_str() == Some(tool.sha256.as_str())
                    })
                });
            if !matches {
                gaps.insert(format!("tool identity mismatch {id}:{}", tool.id));
            }
            // A matching global tool still does not prove execution for this obligation.
            gaps.insert(format!("missing native tool coverage {id}:{}", tool.id));
        }
    }
    if evidence.report().exit_code == 3 {
        gaps.insert("native aggregate incomplete".into());
    }
    ScopeAssessment::Partial {
        gaps: gaps.into_iter().collect(),
    }
}
