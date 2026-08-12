//! Second benchmark reference case (`ROADMAP.md` Phase 6): AFB1-N7-guanine
//! and its ring-opened AFB1-FapyGua counterpart. See `docs/landscape.md`
//! §5 for reference-case selection and `rules/dna_adduct_fragments.json`
//! for the fragmentation rules, sourced from Jaruga et al., ACS Omega
//! 2023, 8(16):14841-14854 (open text: PMC10134230).
//!
//! All expected masses/deltas below are independently hand-computed from
//! molecular formulas against `mass_table`'s own constants (not copied
//! from the paper's rounded values) and cross-checked to match the
//! paper's reported nominal m/z (152, 329, 480, 498, 452) — see the
//! commit introducing this file for the arithmetic.

use adductra::mass_table::Element;
use adductra::{
    AdductCandidate, CandidateAssessment, CandidateGenerator, EvidenceDirection, EvidenceEvaluator,
    EvidenceSet, FragmentEvidenceEvaluator, IonAdductType, IsotopeEvidenceEvaluator, IsotopeLabel,
    MassEvidenceEvaluator, NucleobaseOrigin, Observation, ProductIon, Provenance, Ranker,
    UserSuppliedGenerator, explain,
};

/// AFB1-N7-Gua: C22H17N5O8, [M+H]+ = 480.114989.
fn afb1_n7_gua_observation() -> Observation {
    Observation::new("obs-afb1-n7-gua", 480.114989, 1, IonAdductType::ProtonAdd)
        .unwrap()
        .with_product_ions(vec![
            ProductIon::new(152.056686, Some(80.0)).unwrap(), // protonated guanine (Gua side retains charge)
            ProductIon::new(329.065579, Some(100.0)).unwrap(), // protonated AFB1-diol fragment (AFB1 side retains charge)
        ])
}

fn afb1_n7_gua_candidate() -> AdductCandidate {
    AdductCandidate::from_formula(
        "AFB1-N7-Gua",
        "aflatoxin B1 - N7-guanine adduct",
        "C22H17N5O8",
        Provenance::derived("benchmark-fixture"),
    )
    .unwrap()
    .with_nucleobase_origin(NucleobaseOrigin::Guanine)
}

fn afb1_fapygua_candidate() -> AdductCandidate {
    AdductCandidate::from_formula(
        "AFB1-FapyGua",
        "aflatoxin B1 - formamidopyrimidine (ring-opened) adduct",
        "C22H19N5O9",
        Provenance::derived("benchmark-fixture"),
    )
    .unwrap()
    .with_nucleobase_origin(NucleobaseOrigin::Guanine)
}

#[test]
fn afb1_n7_gua_outranks_its_own_ring_opened_isomer_given_n7_gua_spectrum() {
    // AFB1-FapyGua is not a fabricated decoy — it's a real, distinct,
    // in-vivo interconversion product (N7-Gua + H2O). Given a spectrum
    // that actually matches AFB1-N7-Gua, FapyGua's own precursor mass
    // (18 Da / ~37,500 ppm heavier) must be strongly contradicted, and
    // its own fragment rules (CandidateId-targeted, not
    // NucleobaseOrigin-targeted) must not leak onto the N7-Gua candidate
    // or vice versa.
    let obs = afb1_n7_gua_observation();
    let candidates = vec![afb1_n7_gua_candidate(), afb1_fapygua_candidate()];
    let generated = UserSuppliedGenerator::new(candidates)
        .generate(&obs)
        .unwrap();

    let mass_evaluator = MassEvidenceEvaluator::new(10.0).unwrap();
    let fragment_evaluator = FragmentEvidenceEvaluator::with_built_in_rules().unwrap();

    let assessments: Vec<CandidateAssessment> = generated
        .iter()
        .map(|candidate| {
            let mut evidence = mass_evaluator.evaluate(&obs, candidate).unwrap();
            evidence.extend(fragment_evaluator.evaluate(&obs, candidate).unwrap());
            CandidateAssessment::new(candidate.id.clone(), EvidenceSet::new(evidence))
        })
        .collect();

    let ranked = Ranker::new().rank(assessments);
    assert_eq!(ranked[0].candidate_id, "AFB1-N7-Gua");
    assert_eq!(ranked[1].candidate_id, "AFB1-FapyGua");
    assert!(ranked[0].ranking_score.unwrap() > 0.0);
    assert!(
        ranked[1].ranking_score.unwrap() < 0.0,
        "FapyGua's own [M+H]+ is ~18 Da off this spectrum's precursor and should net-contradict"
    );

    // The two CandidateId-targeted AFB1-N7-Gua fragment rules must both
    // support. The nucleoside-agnostic "Any" deoxyribose-loss rule also
    // fires (it targets every candidate by design) but correctly
    // contradicts: AFB1-N7-Gua is a base-level conjugate with no sugar
    // to lose, so that rule's "not observed" verdict is accurate, if a
    // known scope limitation of "Any" rules assuming nucleoside
    // structure — see ROADMAP.md discovered work.
    let n7_gua_evidence = mass_evaluator
        .evaluate(&obs, &afb1_n7_gua_candidate())
        .unwrap();
    let n7_gua_fragments = fragment_evaluator
        .evaluate(&obs, &afb1_n7_gua_candidate())
        .unwrap();
    assert_eq!(
        n7_gua_fragments.len(),
        3,
        "2 AFB1-specific rules + 1 generic deoxyribose-loss rule"
    );
    let afb1_specific: Vec<_> = n7_gua_fragments
        .iter()
        .filter(|e| e.method().starts_with("rule:afb1-n7-gua"))
        .collect();
    assert_eq!(afb1_specific.len(), 2);
    assert!(
        afb1_specific
            .iter()
            .all(|e| e.direction() == EvidenceDirection::Supporting),
        "{afb1_specific:#?}"
    );
    assert!(
        n7_gua_evidence
            .iter()
            .all(|e| e.direction() == EvidenceDirection::Supporting)
    );

    let explanation = explain(&ranked[0]);
    assert!(explanation.to_text().contains("AFB1-N7-Gua"));
}

#[test]
fn fifteen_n5_labeled_guanine_shift_supported_by_isotope_evidence() {
    // Jaruga et al. synthesized 15N5-labeled internal standards where
    // all 5 nitrogens are on the guanine ring (AFB1 itself has none) —
    // exactly matching the 5 nitrogens in C22H17N5O8's formula, so the
    // count-vs-formula validation added earlier applies at its limit.
    let unlabeled_mz = 480.114989;
    let label = IsotopeLabel::new(Element::N, 15, 5);
    let shift = label.total_shift_da().unwrap();
    let labeled_mz = unlabeled_mz + shift;

    let obs = Observation::new(
        "obs-afb1-n7-gua-15n5",
        labeled_mz,
        1,
        IonAdductType::ProtonAdd,
    )
    .unwrap()
    .with_isotope_labels(vec![label]);

    let isotope_evaluator = IsotopeEvidenceEvaluator::new(0.005).unwrap();
    let evidence = isotope_evaluator
        .evaluate(&obs, &afb1_n7_gua_candidate())
        .unwrap();
    assert_eq!(evidence.len(), 1);
    assert_eq!(evidence[0].direction(), EvidenceDirection::Supporting);

    // A 6th labeled nitrogen is chemically impossible for this formula
    // (only 5 N total, all guanine-derived, none on the AFB1 side).
    let mut bad_obs = obs.clone();
    bad_obs.isotope_labels = vec![IsotopeLabel::new(Element::N, 15, 6)];
    let result = isotope_evaluator.evaluate(&bad_obs, &afb1_n7_gua_candidate());
    assert!(result.is_err());
}
