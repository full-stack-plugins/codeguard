use super::reader::NativeEvidence;
use std::collections::{BTreeMap, BTreeSet};
/// Required targets come from the caller's protected policy, never the native report.
pub struct FrozenObligations(BTreeMap<String, Vec<String>>);
impl FrozenObligations {
    pub(crate) fn frozen_targets(&self) -> BTreeMap<String, Vec<String>> {
        let mut targets = self.0.clone();
        for items in targets.values_mut() {
            items.sort();
        }
        targets
    }
    pub(crate) fn scope_ids(&self) -> Vec<String> {
        self.0.keys().cloned().collect()
    }
    pub fn new(targets: BTreeMap<String, Vec<String>>) -> Result<Self, &'static str> {
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
        Ok(Self(targets))
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
    for (id, targets) in &obligations.0 {
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
    if evidence.report().exit_code == 3 {
        gaps.insert("native aggregate incomplete".into());
    }
    ScopeAssessment::Partial {
        gaps: gaps.into_iter().collect(),
    }
}
