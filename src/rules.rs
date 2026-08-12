//! `AGENTS.md` §13: fragment/neutral-loss knowledge lives as versioned
//! data, not compound-specific `match` arms. A new literature rule is
//! added by editing `rules/*.json`, never by touching evaluator code.

use serde::{Deserialize, Serialize};

use crate::error::AdductraError;
use crate::model::{EvidenceSource, NucleobaseOrigin};

/// Which candidates a rule applies to. Default (externally tagged) serde
/// representation: `"Any"`, `{"NucleobaseOrigin": "Guanine"}`,
/// `{"CandidateId": "some-id"}` — internal tagging can't be used here
/// because `Any` carries no data and can't merge with a tag key.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum RuleTarget {
    Any,
    NucleobaseOrigin(NucleobaseOrigin),
    CandidateId(String),
}

impl RuleTarget {
    pub fn matches(&self, candidate: &crate::model::AdductCandidate) -> bool {
        match self {
            RuleTarget::Any => true,
            RuleTarget::NucleobaseOrigin(origin) => {
                candidate.nucleobase_origin.as_ref() == Some(origin)
            }
            RuleTarget::CandidateId(id) => &candidate.id == id,
        }
    }
}

/// What the rule expects to see, and how close counts as a match.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum RuleExpectation {
    /// An absolute product-ion m/z that should be directly observable.
    DiagnosticFragment { expected_mz: f64, tolerance_da: f64 },
    /// A neutral-loss mass delta from the (charge-corrected) precursor
    /// ion mass. Assumes a singly-charged product ion — ponytail: good
    /// enough for the small-molecule DNA-adduct MS2 this targets in
    /// v0.1; revisit if a benchmark case needs multiply-charged
    /// fragments.
    NeutralLoss {
        expected_delta_da: f64,
        tolerance_da: f64,
    },
}

/// One literature or heuristic fragmentation rule.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FragmentRule {
    pub id: String,
    pub description: String,
    pub target: RuleTarget,
    pub expectation: RuleExpectation,
    pub source: EvidenceSource,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub citation: Option<String>,
    pub version: String,
}

/// The rule set shipped with Adductra (`rules/dna_adduct_fragments.json`),
/// embedded at compile time so evaluating evidence never depends on the
/// filesystem being present (keeps a future WASM build simple, §20).
/// Callers may supply their own rules instead of / in addition to these.
pub fn built_in_rules() -> Result<Vec<FragmentRule>, AdductraError> {
    const RAW: &str = include_str!("../rules/dna_adduct_fragments.json");
    serde_json::from_str(RAW).map_err(|e| AdductraError::InvalidRuleData {
        file: "rules/dna_adduct_fragments.json".to_string(),
        reason: e.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn built_in_rules_parse() {
        let rules = built_in_rules().unwrap();
        assert!(!rules.is_empty());
    }
}
