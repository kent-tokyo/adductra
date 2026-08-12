//! Property-based tests (`AGENTS.md` §16): mass-tolerance/ppm math,
//! isotope-shift consistency, numeric-newtype validation, evidence-set
//! duplication, serialization round-trip, and ranking determinism.

use adductra::mass_table::{Element, isotope_mass, ppm_error};
use adductra::{
    Evidence, EvidenceDetail, EvidenceKind, EvidenceSet, EvidenceSource, EvidenceStrength,
    FiniteF64, IsotopeLabel, NonNegativeF64, Provenance, Ranker,
};
use proptest::prelude::*;

proptest! {
    /// ppm_error is the inverse of "shift theoretical by `delta_ppm`":
    /// recovering `delta_ppm` from the shifted value must round-trip.
    #[test]
    fn ppm_error_round_trips_through_a_synthetic_shift(
        theoretical in 50.0f64..2000.0,
        delta_ppm in -5000.0f64..5000.0,
    ) {
        let observed = theoretical + theoretical * delta_ppm / 1e6;
        let recovered = ppm_error(theoretical, observed);
        prop_assert!((recovered - delta_ppm).abs() < 1e-6, "recovered {recovered} vs {delta_ppm}");
    }

    /// The sign of the ppm error always matches the sign of the raw
    /// (observed - theoretical) difference.
    #[test]
    fn ppm_error_sign_matches_difference_sign(
        theoretical in 50.0f64..2000.0,
        observed in 50.0f64..2000.0,
    ) {
        let err = ppm_error(theoretical, observed);
        prop_assert_eq!(err.signum() as i8, (observed - theoretical).signum() as i8);
    }

    /// `count` labeled atoms always shift mass by exactly `count` times
    /// the per-atom delta; zero labels never shift anything.
    #[test]
    fn isotope_shift_scales_linearly_with_count(count in 0u8..50) {
        let label = IsotopeLabel::new(Element::C, 13, count);
        let shift = label.total_shift_da().unwrap();
        let per_atom = isotope_mass(Element::C, 13).unwrap() - Element::C.monoisotopic_mass();
        prop_assert!((shift - per_atom * count as f64).abs() < 1e-9);
        if count == 0 {
            prop_assert_eq!(shift, 0.0);
        }
    }

    /// FiniteF64 accepts a value if and only if it is finite.
    #[test]
    fn finite_f64_accepts_iff_finite(value in any::<f64>()) {
        let result = FiniteF64::new(value, "x");
        prop_assert_eq!(result.is_ok(), value.is_finite());
    }

    /// NonNegativeF64 accepts a value if and only if it is finite and >= 0.
    #[test]
    fn non_negative_f64_accepts_iff_finite_and_non_negative(value in any::<f64>()) {
        let result = NonNegativeF64::new(value, "x");
        prop_assert_eq!(result.is_ok(), value.is_finite() && value >= 0.0);
    }

    /// N identical copies of the same supporting evidence score exactly N
    /// times a single copy — duplicates are handled predictably, not
    /// specially deduplicated or double-weighted (`AGENTS.md` §16).
    #[test]
    fn duplicate_evidence_scores_linearly(n in 1usize..20) {
        let single = Evidence::supporting(
            EvidenceKind::Mass,
            "t",
            EvidenceDetail::Generic { expected: "1".into(), observed: Some("1".into()) },
            EvidenceStrength::Moderate,
            EvidenceSource::Derived,
            "m",
            Provenance::derived("test"),
        );
        let ranker = Ranker::new();
        let one_score = ranker.score(&EvidenceSet::new(vec![single.clone()]));
        let many_score = ranker.score(&EvidenceSet::new(vec![single; n]));
        prop_assert!((many_score - one_score * n as f64).abs() < 1e-9);
    }

    /// Any Evidence built through the smart constructors survives a JSON
    /// round trip unchanged.
    #[test]
    fn evidence_json_round_trip(
        supporting in any::<bool>(),
        strength_idx in 0u8..3,
    ) {
        let strength = match strength_idx {
            0 => EvidenceStrength::Weak,
            1 => EvidenceStrength::Moderate,
            _ => EvidenceStrength::Strong,
        };
        let detail = EvidenceDetail::Generic { expected: "e".into(), observed: None };
        let e = if supporting {
            Evidence::supporting(EvidenceKind::Mass, "t", detail, strength, EvidenceSource::Derived, "m", Provenance::derived("test"))
        } else {
            Evidence::contradicting(EvidenceKind::Mass, "t", detail, strength, EvidenceSource::Derived, "m", Provenance::derived("test"))
        };
        let json = serde_json::to_string(&e).unwrap();
        let back: Evidence = serde_json::from_str(&json).unwrap();
        prop_assert_eq!(e, back);
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(64))]

    /// Ranking is deterministic across repeated runs, scores never
    /// increase down the list, and equal-score candidates come out in
    /// ascending `candidate_id` order (`AGENTS.md` §16: candidate
    /// ordering determinism). `weights[i]` (0..5 copies of one Strong
    /// supporting evidence, i.e. score = 3.0 * weight) drives each
    /// deliberately-unsorted id's score.
    #[test]
    fn ranking_is_deterministic_and_sorted(weights in prop::collection::vec(0u8..5, 4..5)) {
        use adductra::{CandidateAssessment, EvidenceSet as ES};
        let ids = ["zzz-candidate", "aaa-candidate", "mmm-candidate", "bbb-candidate"];
        let build = || -> Vec<CandidateAssessment> {
            ids.iter()
                .zip(weights.iter())
                .map(|(&id, &w)| {
                    let ev = Evidence::supporting(
                        EvidenceKind::Mass,
                        "t",
                        EvidenceDetail::Generic { expected: "1".into(), observed: Some("1".into()) },
                        EvidenceStrength::Strong,
                        EvidenceSource::Derived,
                        "m",
                        Provenance::derived("test"),
                    );
                    CandidateAssessment::new(id, ES::new(vec![ev; w as usize]))
                })
                .collect()
        };

        let ranker = Ranker::new();
        let ranked_a = ranker.rank(build());
        let ranked_b = ranker.rank(build());

        let ids_a: Vec<_> = ranked_a.iter().map(|a| a.candidate_id.clone()).collect();
        let ids_b: Vec<_> = ranked_b.iter().map(|a| a.candidate_id.clone()).collect();
        prop_assert_eq!(&ids_a, &ids_b, "same input must rank identically every time");

        for pair in ranked_a.windows(2) {
            prop_assert!(pair[0].ranking_score.unwrap() >= pair[1].ranking_score.unwrap());
            if pair[0].ranking_score == pair[1].ranking_score {
                prop_assert!(pair[0].candidate_id < pair[1].candidate_id);
            }
        }
    }
}
