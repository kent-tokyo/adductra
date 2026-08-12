//! Property-based tests (`AGENTS.md` §16): mass-tolerance/ppm math,
//! isotope-shift consistency, numeric-newtype validation, evidence-set
//! duplication, serialization round-trip, ranking determinism, and
//! spectral-library-match similarity invariants.

use adductra::mass_table::{Element, isotope_mass, ppm_error};
use adductra::{
    AdductCandidate, Evidence, EvidenceDetail, EvidenceEvaluator, EvidenceKind, EvidenceSet,
    EvidenceSource, EvidenceStrength, FiniteF64, IonAdductType, IsotopeLabel, NonNegativeF64,
    Observation, ProductIon, Provenance, Ranker, ReferencePeak, ReferenceSpectrum,
    SpectralLibraryEvidenceEvaluator,
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

fn spectral_candidate() -> AdductCandidate {
    AdductCandidate::from_formula("c1", "test", "C10H13N5O5", Provenance::derived("test")).unwrap()
}

/// Runs `SpectralLibraryEvidenceEvaluator` with `reference_peaks` as the
/// sole reference spectrum and `observed_peaks` as the observation's
/// product ions; returns the resulting cosine similarity (`None` if
/// either spectrum has no usable intensity, per the documented fallback).
fn spectral_cosine(reference_peaks: &[(f64, f64)], observed_peaks: &[(f64, f64)]) -> Option<f64> {
    let ref_peaks: Vec<ReferencePeak> = reference_peaks
        .iter()
        .map(|&(mz, i)| ReferencePeak::new(mz, i).unwrap())
        .collect();
    let spectrum =
        ReferenceSpectrum::new("c1", ref_peaks, EvidenceSource::Literature, "1.0.0").unwrap();
    let evaluator = SpectralLibraryEvidenceEvaluator::new(vec![spectrum], 0.01, 0.7).unwrap();
    let mut obs = Observation::new("obs1", 100.0, 1, IonAdductType::ProtonAdd).unwrap();
    obs.product_ions = observed_peaks
        .iter()
        .map(|&(mz, i)| ProductIon::new(mz, Some(i)).unwrap())
        .collect();
    let evidence = evaluator.evaluate(&obs, &spectral_candidate()).unwrap();
    match evidence[0].detail() {
        EvidenceDetail::SpectralLibraryMatch {
            cosine_similarity, ..
        } => cosine_similarity.map(|c| c.get()),
        _ => None,
    }
}

proptest! {
    /// A spectrum compared against itself has cosine similarity 1.0.
    #[test]
    fn spectral_self_similarity_is_one(
        peaks in prop::collection::vec((1.0f64..2000.0, 0.1f64..1000.0), 1..6)
    ) {
        let cosine = spectral_cosine(&peaks, &peaks);
        prop_assert!(cosine.is_some());
        prop_assert!((cosine.unwrap() - 1.0).abs() < 1e-6, "got {:?}", cosine);
    }

    /// Reversing peak order in both spectra can't change the result —
    /// the greedy matcher and the cosine sums are independent of input
    /// order, only of matched (mz, intensity) content.
    #[test]
    fn spectral_cosine_invariant_to_peak_order(
        peaks in prop::collection::vec((1.0f64..2000.0, 0.1f64..1000.0), 2..6)
    ) {
        let forward = spectral_cosine(&peaks, &peaks);
        let mut reversed = peaks.clone();
        reversed.reverse();
        let backward = spectral_cosine(&reversed, &reversed);
        prop_assert!(forward.is_some() && backward.is_some());
        prop_assert!((forward.unwrap() - backward.unwrap()).abs() < 1e-6);
    }

    /// Cosine similarity is invariant to uniformly scaling one spectrum's
    /// intensities by a positive constant (sqrt-transform preserves this:
    /// sqrt(k*x) = sqrt(k)*sqrt(x), a constant factor cosine divides out).
    #[test]
    fn spectral_cosine_invariant_to_intensity_scaling(
        peaks in prop::collection::vec((1.0f64..2000.0, 0.1f64..1000.0), 1..6),
        scale in 0.01f64..100.0,
    ) {
        let scaled: Vec<(f64, f64)> = peaks.iter().map(|&(mz, i)| (mz, i * scale)).collect();
        let unscaled_self = spectral_cosine(&peaks, &peaks);
        let scaled_vs_unscaled = spectral_cosine(&scaled, &peaks);
        prop_assert!(unscaled_self.is_some() && scaled_vs_unscaled.is_some());
        prop_assert!((unscaled_self.unwrap() - scaled_vs_unscaled.unwrap()).abs() < 1e-6);
    }

    /// All-zero-intensity peak lists must fall back to `None` (no
    /// cosine), never produce NaN -- the zero-norm guard.
    #[test]
    fn spectral_all_zero_intensity_never_nan(
        mzs in prop::collection::vec(1.0f64..2000.0, 1..6)
    ) {
        let peaks: Vec<(f64, f64)> = mzs.iter().map(|&mz| (mz, 0.0)).collect();
        let cosine = spectral_cosine(&peaks, &peaks);
        prop_assert!(cosine.is_none());
    }

    /// Regression guard for the greedy-matcher double-counting bug: the
    /// matched-peak count can never exceed either spectrum's own size.
    #[test]
    fn spectral_matched_peak_count_never_exceeds_either_spectrum(
        reference in prop::collection::vec((1.0f64..2000.0, 0.0f64..1000.0), 1..8),
        observed in prop::collection::vec((1.0f64..2000.0, 0.0f64..1000.0), 1..8),
    ) {
        let ref_peaks: Vec<ReferencePeak> = reference
            .iter()
            .map(|&(mz, i)| ReferencePeak::new(mz, i).unwrap())
            .collect();
        let spectrum =
            ReferenceSpectrum::new("c1", ref_peaks, EvidenceSource::Literature, "1.0.0").unwrap();
        let evaluator = SpectralLibraryEvidenceEvaluator::new(vec![spectrum], 0.01, 0.7).unwrap();
        let mut obs = Observation::new("obs1", 100.0, 1, IonAdductType::ProtonAdd).unwrap();
        obs.product_ions = observed
            .iter()
            .map(|&(mz, i)| ProductIon::new(mz, Some(i)).unwrap())
            .collect();
        let evidence = evaluator.evaluate(&obs, &spectral_candidate()).unwrap();
        if let EvidenceDetail::SpectralLibraryMatch { matched_peak_count, reference_peak_count, .. } = evidence[0].detail() {
            prop_assert_eq!(*reference_peak_count, reference.len() as u32);
            prop_assert!(*matched_peak_count <= reference.len() as u32);
            prop_assert!(*matched_peak_count <= observed.len() as u32);
        }
    }
}
