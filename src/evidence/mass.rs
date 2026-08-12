//! `AGENTS.md` §7 P0: exact mass and precursor consistency. Produces two
//! evidence items per candidate:
//!
//! - `EvidenceKind::Mass` — theoretical vs. observed **neutral** mass (Da),
//!   i.e. "does this candidate's formula match, once the ionization is
//!   undone."
//! - `EvidenceKind::PrecursorConsistency` — theoretical vs. observed
//!   **ion** m/z (the conventional mass-spec ppm-accuracy figure), plus a
//!   hard check that the stated charge sign matches the stated ion
//!   adduct's polarity.

use crate::error::AdductraError;
use crate::evaluator::EvidenceEvaluator;
use crate::mass_table::{Formula, ppm_error};
use crate::model::{
    AdductCandidate, Evidence, EvidenceDetail, EvidenceKind, EvidenceSource, EvidenceStrength,
    FiniteF64, NonNegativeF64, Observation, Provenance,
};

/// ponytail: single global tolerance for both mass and precursor
/// evidence, and a fixed 0.5x/2x strength-banding heuristic. Good enough
/// for a transparent v0.1 baseline (`AGENTS.md` §10); revisit with
/// per-evidence-kind tolerances if benchmark data (Phase 6) shows the
/// bands are miscalibrated.
pub struct MassEvidenceEvaluator {
    tolerance_ppm: f64,
}

impl MassEvidenceEvaluator {
    pub fn new(tolerance_ppm: f64) -> Result<Self, AdductraError> {
        NonNegativeF64::new(tolerance_ppm, "tolerance_ppm")?;
        Ok(Self { tolerance_ppm })
    }

    fn strength_for(&self, abs_delta_ppm: f64, within_tolerance: bool) -> EvidenceStrength {
        if within_tolerance {
            if abs_delta_ppm <= self.tolerance_ppm / 2.0 {
                EvidenceStrength::Strong
            } else {
                EvidenceStrength::Moderate
            }
        } else if abs_delta_ppm <= self.tolerance_ppm * 2.0 {
            EvidenceStrength::Weak
        } else {
            EvidenceStrength::Strong
        }
    }

    fn provenance(&self) -> Provenance {
        Provenance::derived("mass_evidence_evaluator_v1")
            .with_parameter("tolerance_ppm", self.tolerance_ppm)
    }
}

/// Expected sign of `charge` for each ion adduct type (+1 = positive
/// mode, -1 = negative mode). `Custom` adducts are inferred from the
/// sign of their mass shift.
fn expected_charge_sign(adduct: &crate::model::IonAdductType) -> i8 {
    use crate::model::IonAdductType::*;
    match adduct {
        ProtonAdd | SodiumAdd | PotassiumAdd | AmmoniumAdd => 1,
        ProtonLoss => -1,
        Custom { mass_shift_da, .. } => {
            if *mass_shift_da >= 0.0 {
                1
            } else {
                -1
            }
        }
    }
}

impl EvidenceEvaluator for MassEvidenceEvaluator {
    fn evaluate(
        &self,
        observation: &Observation,
        candidate: &AdductCandidate,
    ) -> Result<Vec<Evidence>, AdductraError> {
        if observation.charge == 0 {
            return Err(AdductraError::InvalidCharge(0));
        }

        let formula = Formula::parse(&candidate.formula)?;
        let mut isotope_shift_da = 0.0;
        for label in &observation.isotope_labels {
            let available = formula.count(label.element);
            if label.count as u32 > available {
                return Err(AdductraError::ImpossibleIsotopeCount {
                    element: label.element.symbol().to_string(),
                    requested: label.count,
                    available,
                });
            }
            isotope_shift_da += label.total_shift_da()?;
        }
        let theoretical_neutral = formula.monoisotopic_mass() + isotope_shift_da;

        let z = observation.charge.unsigned_abs() as f64;
        // Assumes homogeneous adduct stacking: a charge of z carries z
        // instances of the same ionization event (e.g. [M+2H]2+ = two
        // protons), which is the standard mass-spec convention for
        // multiply protonated/deprotonated species and the common case
        // for small-molecule DNA adducts. `ion_shift` is therefore the
        // *per-charge* contribution, scaled by z below — NOT a single
        // fixed adduct mass regardless of charge state.
        let ion_shift = observation.ion_adduct.mass_shift_da() * z;
        let observed_ion_mass = observation.precursor_mz.get() * z;
        let observed_neutral = observed_ion_mass - ion_shift;

        let mut evidence = Vec::with_capacity(2);

        // --- Mass evidence: theoretical vs. observed neutral mass (Da) ---
        let mass_delta_ppm = ppm_error(theoretical_neutral, observed_neutral);
        let mass_within = mass_delta_ppm.abs() <= self.tolerance_ppm;
        let mass_detail = EvidenceDetail::Mass {
            theoretical_da: FiniteF64::new(theoretical_neutral, "theoretical_da")?,
            observed_da: FiniteF64::new(observed_neutral, "observed_da")?,
            delta_ppm: FiniteF64::new(mass_delta_ppm, "delta_ppm")?,
            tolerance_ppm: NonNegativeF64::new(self.tolerance_ppm, "tolerance_ppm")?,
        };
        let strength = self.strength_for(mass_delta_ppm.abs(), mass_within);
        evidence.push(if mass_within {
            Evidence::supporting(
                EvidenceKind::Mass,
                format!("neutral monoisotopic mass of {}", candidate.formula),
                mass_detail,
                strength,
                EvidenceSource::Derived,
                "candidate formula mass (mass_table) vs precursor-derived neutral mass",
                self.provenance(),
            )
        } else {
            Evidence::contradicting(
                EvidenceKind::Mass,
                format!("neutral monoisotopic mass of {}", candidate.formula),
                mass_detail,
                strength,
                EvidenceSource::Derived,
                "candidate formula mass (mass_table) vs precursor-derived neutral mass",
                self.provenance(),
            )
        });

        // --- Precursor consistency: theoretical vs observed ion m/z, ---
        // --- plus charge-sign / adduct-polarity plausibility.        ---
        let theoretical_ion_mass = theoretical_neutral + ion_shift;
        let theoretical_mz = theoretical_ion_mass / z;
        let mz_delta_ppm = ppm_error(theoretical_mz, observation.precursor_mz.get());
        let polarity_ok =
            observation.charge.signum() == expected_charge_sign(&observation.ion_adduct);
        let mz_within = polarity_ok && mz_delta_ppm.abs() <= self.tolerance_ppm;
        let precursor_detail = EvidenceDetail::PrecursorConsistency {
            expected_mz: FiniteF64::new(theoretical_mz, "expected_mz")?,
            observed_mz: FiniteF64::new(observation.precursor_mz.get(), "observed_mz")?,
            delta_ppm: FiniteF64::new(mz_delta_ppm, "delta_ppm")?,
            tolerance_ppm: NonNegativeF64::new(self.tolerance_ppm, "tolerance_ppm")?,
            charge: observation.charge,
            ion_adduct: observation.ion_adduct.label(),
        };
        let precursor_strength = if !polarity_ok {
            EvidenceStrength::Strong
        } else {
            self.strength_for(mz_delta_ppm.abs(), mz_within)
        };
        evidence.push(if mz_within {
            Evidence::supporting(
                EvidenceKind::PrecursorConsistency,
                format!(
                    "precursor m/z under charge {} / {}",
                    observation.charge,
                    observation.ion_adduct.label()
                ),
                precursor_detail,
                precursor_strength,
                EvidenceSource::Derived,
                "theoretical ion m/z (mass_table) vs observed precursor m/z",
                self.provenance(),
            )
        } else {
            Evidence::contradicting(
                EvidenceKind::PrecursorConsistency,
                format!(
                    "precursor m/z under charge {} / {}",
                    observation.charge,
                    observation.ion_adduct.label()
                ),
                precursor_detail,
                precursor_strength,
                EvidenceSource::Derived,
                "theoretical ion m/z (mass_table) vs observed precursor m/z",
                self.provenance(),
            )
        });

        Ok(evidence)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mass_table::{Element, PROTON_MASS};
    use crate::model::{IonAdductType, IsotopeLabel, NucleobaseOrigin};

    fn eight_oxo_dg_base_candidate() -> AdductCandidate {
        AdductCandidate::from_formula(
            "8oxoGua",
            "8-oxoguanine (base ion)",
            "C5H5N5O2",
            Provenance::derived("fixture"),
        )
        .unwrap()
        .with_nucleobase_origin(NucleobaseOrigin::Guanine)
    }

    #[test]
    fn matching_precursor_supports_both_evidence_items() {
        let obs = Observation::new("obs1", 168.0511, 1, IonAdductType::ProtonAdd).unwrap();
        let candidate = eight_oxo_dg_base_candidate();
        let evaluator = MassEvidenceEvaluator::new(10.0).unwrap();
        let evidence = evaluator.evaluate(&obs, &candidate).unwrap();
        assert_eq!(evidence.len(), 2);
        assert!(
            evidence
                .iter()
                .all(|e| e.direction() == crate::model::EvidenceDirection::Supporting)
        );
    }

    #[test]
    fn mass_close_decoy_is_contradicted_outside_tolerance() {
        // A decoy ~0.05 Da off (~300 ppm at this mass) should be rejected
        // at a tight 10 ppm tolerance.
        let obs = Observation::new("obs1", 168.0511, 1, IonAdductType::ProtonAdd).unwrap();
        let decoy = AdductCandidate::from_formula(
            "decoy1",
            "mass-close decoy",
            "C5H6N4O2", // deliberately different formula, close mass
            Provenance::derived("fixture"),
        )
        .unwrap();
        let evaluator = MassEvidenceEvaluator::new(10.0).unwrap();
        let evidence = evaluator.evaluate(&obs, &decoy).unwrap();
        let mass_ev = evidence
            .iter()
            .find(|e| *e.kind() == EvidenceKind::Mass)
            .unwrap();
        assert_eq!(
            mass_ev.direction(),
            crate::model::EvidenceDirection::Contradicting
        );
    }

    #[test]
    fn polarity_mismatch_forces_contradicting_precursor_evidence() {
        // Negative charge with a positive-mode adduct is internally
        // inconsistent regardless of how close the ppm happens to be.
        let obs = Observation::new("obs1", 168.0511, -1, IonAdductType::ProtonAdd).unwrap();
        let candidate = eight_oxo_dg_base_candidate();
        let evaluator = MassEvidenceEvaluator::new(10.0).unwrap();
        let evidence = evaluator.evaluate(&obs, &candidate).unwrap();
        let precursor_ev = evidence
            .iter()
            .find(|e| *e.kind() == EvidenceKind::PrecursorConsistency)
            .unwrap();
        assert_eq!(
            precursor_ev.direction(),
            crate::model::EvidenceDirection::Contradicting
        );
        assert_eq!(precursor_ev.strength(), Some(EvidenceStrength::Strong));
    }

    #[test]
    fn zero_charge_is_rejected_not_panicking() {
        let mut obs = Observation::new("obs1", 168.0511, 1, IonAdductType::ProtonAdd).unwrap();
        obs.charge = 0; // simulate a hand-built/deserialized bad Observation
        let candidate = eight_oxo_dg_base_candidate();
        let evaluator = MassEvidenceEvaluator::new(10.0).unwrap();
        assert!(evaluator.evaluate(&obs, &candidate).is_err());
    }

    #[test]
    fn doubly_charged_precursor_scales_adduct_shift_by_charge() {
        // Regression for a bug where `[M+2H]2+` subtracted only one
        // proton's mass instead of two, producing a ~3600 ppm error that
        // no z=1 test could catch.
        let candidate = eight_oxo_dg_base_candidate();
        let neutral = candidate.monoisotopic_mass().unwrap();
        let z: i8 = 2;
        let mz = (neutral + 2.0 * PROTON_MASS) / z as f64;
        let obs = Observation::new("obs-z2", mz, z, IonAdductType::ProtonAdd).unwrap();
        let evaluator = MassEvidenceEvaluator::new(10.0).unwrap();
        let evidence = evaluator.evaluate(&obs, &candidate).unwrap();
        assert!(
            evidence
                .iter()
                .all(|e| e.direction() == crate::model::EvidenceDirection::Supporting),
            "{evidence:#?}"
        );
    }

    #[test]
    fn isotope_count_exceeding_candidate_formula_is_rejected() {
        // C5H5N5O2 has only 5 nitrogens; requesting 6 labeled 15N atoms
        // is chemically impossible and must error, not silently shift
        // the theoretical mass by a fabricated amount (§17).
        let candidate = eight_oxo_dg_base_candidate();
        let obs = Observation::new("obs-iso", 168.0511, 1, IonAdductType::ProtonAdd)
            .unwrap()
            .with_isotope_labels(vec![IsotopeLabel::new(Element::N, 15, 6)]);
        let evaluator = MassEvidenceEvaluator::new(10.0).unwrap();
        let result = evaluator.evaluate(&obs, &candidate);
        assert!(matches!(
            result,
            Err(AdductraError::ImpossibleIsotopeCount { .. })
        ));
    }
}
