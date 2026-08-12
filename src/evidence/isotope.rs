//! `AGENTS.md` §7 P1: isotope-labeling evidence, as its own falsifiable
//! check independent of `EvidenceKind::Mass` (which folds the expected
//! isotope shift into its theoretical mass but never checks the shift
//! *itself* for consistency). This evaluator asks a narrower question:
//! "does the observed mass shift, relative to the unlabeled candidate,
//! match the expected shift for the stated label(s)?"
//!
//! ponytail: with more than one `IsotopeLabel` on an `Observation`
//! (e.g. both ¹³C and ¹⁵N used together), a single precursor m/z can't
//! be decomposed back into a per-label contribution — there's only one
//! number to compare against the *sum* of expected shifts. This
//! evaluator therefore produces one evidence item per observation, not
//! one per label. Per-label attribution would need per-fragment isotope
//! pattern analysis, deferred until a benchmark case needs it.

use crate::error::AdductraError;
use crate::evaluator::{EvidenceEvaluator, tolerance_strength};
use crate::mass_table::Formula;
use crate::model::{
    AdductCandidate, Evidence, EvidenceDetail, EvidenceKind, EvidenceSource, FiniteF64,
    NonNegativeF64, Observation, Provenance,
};

/// Evaluates isotope-labeling evidence: does the observed mass shift
/// match the sum of expected shifts for an [`Observation`]'s
/// [`crate::model::IsotopeLabel`]s?
pub struct IsotopeEvidenceEvaluator {
    tolerance_da: f64,
}

impl IsotopeEvidenceEvaluator {
    /// `tolerance_da` is an absolute Daltons tolerance (not ppm): isotope
    /// shifts are small, roughly fixed quantities regardless of the
    /// candidate's overall mass, so a relative (ppm) tolerance would
    /// either be meaninglessly loose at small shifts or blow up near a
    /// zero expected shift.
    pub fn new(tolerance_da: f64) -> Result<Self, AdductraError> {
        NonNegativeF64::new(tolerance_da, "tolerance_da")?;
        Ok(Self { tolerance_da })
    }

    fn provenance(&self) -> Provenance {
        Provenance::derived("isotope_evidence_evaluator_v1")
            .with_parameter("tolerance_da", self.tolerance_da)
    }
}

impl EvidenceEvaluator for IsotopeEvidenceEvaluator {
    fn evaluate(
        &self,
        observation: &Observation,
        candidate: &AdductCandidate,
    ) -> Result<Vec<Evidence>, AdductraError> {
        if observation.isotope_labels.is_empty() {
            let detail = EvidenceDetail::IsotopeLabel {
                expected_shift_da: FiniteF64::new(0.0, "expected_shift_da")?,
                tolerance_da: NonNegativeF64::new(self.tolerance_da, "tolerance_da")?,
                observed_shift_da: None,
                label_count: 0,
            };
            return Ok(vec![Evidence::not_applicable(
                EvidenceKind::IsotopeLabel,
                "isotope labeling",
                detail,
                EvidenceSource::Derived,
                "no isotope label present on this observation",
                self.provenance(),
            )]);
        }

        let formula = Formula::parse(&candidate.formula)?;
        let expected_shift_da = observation.total_isotope_shift_da(&formula)?;
        let observed_neutral = observation.observed_neutral_mass()?;
        let observed_shift_da = observed_neutral - formula.monoisotopic_mass();

        let delta = (observed_shift_da - expected_shift_da).abs();
        let within = delta <= self.tolerance_da;
        let label_count = observation
            .isotope_labels
            .iter()
            .map(|l| l.count as u32)
            .sum::<u32>()
            .min(u8::MAX as u32) as u8;

        let detail = EvidenceDetail::IsotopeLabel {
            expected_shift_da: FiniteF64::new(expected_shift_da, "expected_shift_da")?,
            tolerance_da: NonNegativeF64::new(self.tolerance_da, "tolerance_da")?,
            observed_shift_da: Some(FiniteF64::new(observed_shift_da, "observed_shift_da")?),
            label_count,
        };
        let strength = tolerance_strength(delta, self.tolerance_da, within);
        let what_was_tested = format!("isotope shift for {label_count} labelled atom(s)");
        let method =
            "observed precursor-derived shift (mass_table) vs sum of expected label shifts";

        Ok(vec![if within {
            Evidence::supporting(
                EvidenceKind::IsotopeLabel,
                what_was_tested,
                detail,
                strength,
                EvidenceSource::Derived,
                method,
                self.provenance(),
            )
        } else {
            Evidence::contradicting(
                EvidenceKind::IsotopeLabel,
                what_was_tested,
                detail,
                strength,
                EvidenceSource::Derived,
                method,
                self.provenance(),
            )
        }])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mass_table::Element;
    use crate::model::{EvidenceDirection, IonAdductType, IsotopeLabel, Provenance as Prov};

    fn eight_oxo_dg_base_candidate() -> AdductCandidate {
        AdductCandidate::from_formula(
            "8oxoGua",
            "8-oxoguanine (base ion)",
            "C5H5N5O2",
            Prov::derived("fixture"),
        )
        .unwrap()
    }

    #[test]
    fn no_labels_is_not_applicable() {
        let obs = Observation::new("obs1", 168.0511, 1, IonAdductType::ProtonAdd).unwrap();
        let evaluator = IsotopeEvidenceEvaluator::new(0.005).unwrap();
        let evidence = evaluator
            .evaluate(&obs, &eight_oxo_dg_base_candidate())
            .unwrap();
        assert_eq!(evidence.len(), 1);
        assert_eq!(evidence[0].direction(), EvidenceDirection::NotApplicable);
    }

    #[test]
    fn matching_shift_supports() {
        // 2 atoms of 15N: expected shift ~1.994 Da (2 * 0.997035).
        let label = IsotopeLabel::new(Element::N, 15, 2);
        let shift = label.total_shift_da().unwrap();
        let unlabeled_neutral = 167.044325; // C5H5N5O2, see mass_table tests
        let mz = unlabeled_neutral + shift + crate::mass_table::PROTON_MASS;
        let obs = Observation::new("obs1", mz, 1, IonAdductType::ProtonAdd)
            .unwrap()
            .with_isotope_labels(vec![label]);
        let evaluator = IsotopeEvidenceEvaluator::new(0.005).unwrap();
        let evidence = evaluator
            .evaluate(&obs, &eight_oxo_dg_base_candidate())
            .unwrap();
        assert_eq!(evidence.len(), 1);
        assert_eq!(evidence[0].direction(), EvidenceDirection::Supporting);
    }

    #[test]
    fn wrong_shift_contradicts() {
        // Precursor mass matches the *unlabeled* candidate exactly, but
        // the observation claims 2 labelled 15N atoms (~2 Da expected
        // shift that never shows up).
        let unlabeled_neutral = 167.044325;
        let mz = unlabeled_neutral + crate::mass_table::PROTON_MASS;
        let obs = Observation::new("obs1", mz, 1, IonAdductType::ProtonAdd)
            .unwrap()
            .with_isotope_labels(vec![IsotopeLabel::new(Element::N, 15, 2)]);
        let evaluator = IsotopeEvidenceEvaluator::new(0.005).unwrap();
        let evidence = evaluator
            .evaluate(&obs, &eight_oxo_dg_base_candidate())
            .unwrap();
        assert_eq!(evidence[0].direction(), EvidenceDirection::Contradicting);
    }

    #[test]
    fn impossible_label_count_is_rejected() {
        // C5H5N5O2 has only 5 nitrogens.
        let obs = Observation::new("obs1", 168.0511, 1, IonAdductType::ProtonAdd)
            .unwrap()
            .with_isotope_labels(vec![IsotopeLabel::new(Element::N, 15, 6)]);
        let evaluator = IsotopeEvidenceEvaluator::new(0.005).unwrap();
        let result = evaluator.evaluate(&obs, &eight_oxo_dg_base_candidate());
        assert!(matches!(
            result,
            Err(AdductraError::ImpossibleIsotopeCount { .. })
        ));
    }
}
