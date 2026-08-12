//! `AGENTS.md` §7 P0 (diagnostic fragments, neutral losses) + §13 (rule
//! data, not hard-coded chemistry). One evaluator covers both
//! `EvidenceKind::DiagnosticFragment` and `EvidenceKind::NeutralLoss` —
//! both reduce to "does an observed product ion land within tolerance of
//! an expected m/z," just computed differently — driven entirely by
//! [`FragmentRule`] data. Adding a new literature rule never touches this
//! file.
//!
//! ponytail: `NeutralLoss` matching only checks that the *spectrum* shows
//! the expected loss relative to the precursor — it does not verify the
//! *candidate's own formula* actually contains the lost fragment's atoms
//! (e.g. that a deoxyribose-loss candidate even has a deoxyribose-like
//! substructure). This mirrors how untargeted neutral-loss screening
//! tools such as nLossFinder work in practice (`docs/landscape.md` §1),
//! so two candidates with the same precursor get identical NeutralLoss
//! evidence regardless of whether their structure explains it. Add
//! formula-subtraction verification (candidate formula ⊇ lost-fragment
//! formula) when a benchmark case needs it to discriminate.

use crate::error::AdductraError;
use crate::evaluator::EvidenceEvaluator;
use crate::model::{
    AdductCandidate, Evidence, EvidenceDetail, EvidenceKind, EvidenceStrength, FiniteF64,
    MissingReason, NonNegativeF64, Observation, Provenance,
};
use crate::rules::{FragmentRule, RuleExpectation, built_in_rules};

/// Evaluates diagnostic-fragment and neutral-loss evidence from a
/// [`FragmentRule`] set (built-in or caller-supplied).
pub struct FragmentEvidenceEvaluator {
    rules: Vec<FragmentRule>,
}

impl FragmentEvidenceEvaluator {
    pub fn new(rules: Vec<FragmentRule>) -> Self {
        Self { rules }
    }

    pub fn with_built_in_rules() -> Result<Self, AdductraError> {
        Ok(Self::new(built_in_rules()?))
    }
}

fn find_matching_ion(observation: &Observation, target_mz: f64, tolerance_da: f64) -> Option<f64> {
    observation
        .product_ions
        .iter()
        .map(|ion| ion.mz.get())
        .find(|&mz| (mz - target_mz).abs() <= tolerance_da)
}

fn rule_provenance(rule: &FragmentRule) -> Provenance {
    Provenance {
        software_version: env!("CARGO_PKG_VERSION").to_string(),
        rule_version: Some(rule.version.clone()),
        source_citation: rule.citation.clone(),
        algorithm_version: Some("fragment_evidence_evaluator_v1".to_string()),
        ..Default::default()
    }
    .with_parameter("rule_id", rule.id.clone())
}

impl EvidenceEvaluator for FragmentEvidenceEvaluator {
    fn evaluate(
        &self,
        observation: &Observation,
        candidate: &AdductCandidate,
    ) -> Result<Vec<Evidence>, AdductraError> {
        if observation.charge == 0 {
            return Err(AdductraError::InvalidCharge(0));
        }
        let z = observation.charge.unsigned_abs() as f64;
        let parent_ion_mass = observation.precursor_mz.get() * z;
        let has_ms2 = !observation.product_ions.is_empty();

        let mut evidence = Vec::new();
        for rule in self.rules.iter().filter(|r| r.target.matches(candidate)) {
            let provenance = rule_provenance(rule);
            let ev = match &rule.expectation {
                RuleExpectation::DiagnosticFragment {
                    expected_mz,
                    tolerance_da,
                } => {
                    let matched = find_matching_ion(observation, *expected_mz, *tolerance_da);
                    let detail = EvidenceDetail::DiagnosticFragment {
                        expected_mz: FiniteF64::new(*expected_mz, "expected_mz")?,
                        tolerance_da: NonNegativeF64::new(*tolerance_da, "tolerance_da")?,
                        matched_mz: matched
                            .map(|m| FiniteF64::new(m, "matched_mz"))
                            .transpose()?,
                    };
                    build_evidence(
                        EvidenceKind::DiagnosticFragment,
                        rule,
                        detail,
                        matched.is_some(),
                        has_ms2,
                        provenance,
                    )
                }
                RuleExpectation::NeutralLoss {
                    expected_delta_da,
                    tolerance_da,
                } => {
                    let expected_fragment_mz = parent_ion_mass - expected_delta_da;
                    let matched =
                        find_matching_ion(observation, expected_fragment_mz, *tolerance_da);
                    let observed_delta_da = matched.map(|m| parent_ion_mass - m);
                    let detail = EvidenceDetail::NeutralLoss {
                        expected_delta_da: FiniteF64::new(*expected_delta_da, "expected_delta_da")?,
                        tolerance_da: NonNegativeF64::new(*tolerance_da, "tolerance_da")?,
                        observed_delta_da: observed_delta_da
                            .map(|d| FiniteF64::new(d, "observed_delta_da"))
                            .transpose()?,
                    };
                    build_evidence(
                        EvidenceKind::NeutralLoss,
                        rule,
                        detail,
                        matched.is_some(),
                        has_ms2,
                        provenance,
                    )
                }
            };
            evidence.push(ev);
        }
        Ok(evidence)
    }
}

/// §25: absence of evidence is not evidence of absence. No MS2 data at
/// all → `Missing(NotMeasured)`. MS2 data present but nothing landed
/// within tolerance → `Contradicting` ("measured but absent" counts
/// against the candidate, per `AGENTS.md` §25's own distinction).
fn build_evidence(
    kind: EvidenceKind,
    rule: &FragmentRule,
    detail: EvidenceDetail,
    matched: bool,
    has_ms2: bool,
    provenance: Provenance,
) -> Evidence {
    let source = rule.source;
    let method = format!("rule:{}", rule.id);
    if matched {
        Evidence::supporting(
            kind,
            rule.description.clone(),
            detail,
            EvidenceStrength::Strong,
            source,
            method,
            provenance,
        )
    } else if !has_ms2 {
        Evidence::missing(
            kind,
            rule.description.clone(),
            detail,
            MissingReason::NotMeasured,
            source,
            method,
            provenance,
        )
    } else {
        Evidence::contradicting(
            kind,
            rule.description.clone(),
            detail,
            EvidenceStrength::Moderate,
            source,
            method,
            provenance,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{
        AdductCandidate, EvidenceDirection, IonAdductType, NucleobaseOrigin, ProductIon,
    };

    fn eight_oxo_dg_candidate() -> AdductCandidate {
        AdductCandidate::from_formula(
            "8oxodG",
            "8-oxo-2'-deoxyguanosine",
            "C10H13N5O5",
            Provenance::derived("fixture"),
        )
        .unwrap()
        .with_nucleobase_origin(NucleobaseOrigin::Other("8-oxo-guanine".to_string()))
    }

    fn full_ms2_observation() -> Observation {
        Observation::new("obs1", 284.0989, 1, IonAdductType::ProtonAdd)
            .unwrap()
            .with_product_ions(vec![
                ProductIon::new(168.0516, Some(100.0)).unwrap(),
                ProductIon::new(140.0567, Some(40.0)).unwrap(),
                ProductIon::new(112.0618, Some(15.0)).unwrap(),
            ])
    }

    #[test]
    fn all_rules_support_when_full_ms2_matches() {
        let evaluator = FragmentEvidenceEvaluator::with_built_in_rules().unwrap();
        let evidence = evaluator
            .evaluate(&full_ms2_observation(), &eight_oxo_dg_candidate())
            .unwrap();
        assert_eq!(evidence.len(), 3);
        assert!(
            evidence
                .iter()
                .all(|e| e.direction() == EvidenceDirection::Supporting)
        );
    }

    #[test]
    fn no_ms2_data_is_missing_not_contradicting() {
        let obs = Observation::new("obs1", 284.0989, 1, IonAdductType::ProtonAdd).unwrap();
        let evaluator = FragmentEvidenceEvaluator::with_built_in_rules().unwrap();
        let evidence = evaluator.evaluate(&obs, &eight_oxo_dg_candidate()).unwrap();
        assert!(
            evidence
                .iter()
                .all(|e| e.direction() == EvidenceDirection::Missing)
        );
    }

    #[test]
    fn ms2_present_but_fragment_absent_is_contradicting() {
        // MS2 was acquired but only the base ion shows up; the two CO-loss
        // fragments are genuinely absent, not merely unmeasured.
        let obs = Observation::new("obs1", 284.0989, 1, IonAdductType::ProtonAdd)
            .unwrap()
            .with_product_ions(vec![ProductIon::new(168.0516, Some(100.0)).unwrap()]);
        let evaluator = FragmentEvidenceEvaluator::with_built_in_rules().unwrap();
        let evidence = evaluator.evaluate(&obs, &eight_oxo_dg_candidate()).unwrap();
        let co_loss_1 = evidence
            .iter()
            .find(|e| *e.kind() == EvidenceKind::DiagnosticFragment)
            .unwrap();
        assert_eq!(co_loss_1.direction(), EvidenceDirection::Contradicting);
    }

    #[test]
    fn wrong_nucleobase_origin_skips_guanine_specific_rules() {
        let candidate = AdductCandidate::from_formula(
            "adenine-adduct",
            "some adenine-derived adduct",
            "C10H13N5O4",
            Provenance::derived("fixture"),
        )
        .unwrap()
        .with_nucleobase_origin(NucleobaseOrigin::Adenine);
        let evaluator = FragmentEvidenceEvaluator::with_built_in_rules().unwrap();
        let evidence = evaluator
            .evaluate(&full_ms2_observation(), &candidate)
            .unwrap();
        // Only the nucleobase-agnostic deoxyribose-loss rule applies.
        assert_eq!(evidence.len(), 1);
        assert_eq!(*evidence[0].kind(), EvidenceKind::NeutralLoss);
    }
}
