//! End-to-end demonstration of Adductra's v0.1 milestone
//! (`AGENTS.md`, "最初のゴール"): given a known DNA adduct plus competing
//! decoys, evaluate exact-mass / MS2 / neutral-loss evidence, rank the
//! candidates, and explain the ranking.
//!
//! Reference case: 8-oxo-2'-deoxyguanosine (`docs/landscape.md` §5),
//! [M+H]+ precursor m/z 284.0989 with diagnostic fragments at m/z
//! 168.0516 / 140.0567 / 112.0618 (sequential CO loss from the 8-oxo
//! base ion after deoxyribose loss).

use adductra::{
    AdductCandidate, AdductReport, CandidateAssessment, CandidateGenerator, EvidenceEvaluator,
    EvidenceSet, FragmentEvidenceEvaluator, IonAdductType, MassEvidenceEvaluator, NucleobaseOrigin,
    Observation, ProductIon, Provenance, Ranker, UserSuppliedGenerator, explain,
};

fn observation() -> Observation {
    Observation::new("obs-8oxodg-1", 284.0989, 1, IonAdductType::ProtonAdd)
        .unwrap()
        .with_product_ions(vec![
            ProductIon::new(168.0516, Some(100.0)).unwrap(),
            ProductIon::new(140.0567, Some(40.0)).unwrap(),
            ProductIon::new(112.0618, Some(15.0)).unwrap(),
        ])
}

#[test]
fn known_adduct_outranks_decoys_with_explainable_evidence() {
    let obs = observation();

    let correct = AdductCandidate::from_formula(
        "8-oxo-dG",
        "8-oxo-2'-deoxyguanosine",
        "C10H13N5O5",
        Provenance::derived("benchmark-fixture"),
    )
    .unwrap()
    .with_nucleobase_origin(NucleobaseOrigin::Guanine);

    // Same formula/mass as the correct candidate (an isomer), but tagged
    // with the wrong nucleobase origin — the guanine-specific CO-loss
    // rules won't apply to it, so it accumulates less corroborating
    // evidence even though its mass matches.
    let isomeric_decoy = AdductCandidate::from_formula(
        "adenine-isomer",
        "isomeric adenine-derived decoy (same formula)",
        "C10H13N5O5",
        Provenance::derived("benchmark-fixture"),
    )
    .unwrap()
    .with_nucleobase_origin(NucleobaseOrigin::Adenine);

    // A genuinely wrong candidate: nominally mass-close (283 vs 283) but
    // ~129 ppm off on exact mass — an O-for-CH4 near-isobaric swap.
    let mass_close_decoy = AdductCandidate::from_formula(
        "mass-close-decoy",
        "near-isobaric wrong-formula decoy",
        "C11H17N5O4",
        Provenance::derived("benchmark-fixture"),
    )
    .unwrap();

    let generator = UserSuppliedGenerator::new(vec![
        correct.clone(),
        isomeric_decoy.clone(),
        mass_close_decoy.clone(),
    ]);
    let candidates = generator.generate(&obs).unwrap();

    let mass_evaluator = MassEvidenceEvaluator::new(10.0).unwrap();
    let fragment_evaluator = FragmentEvidenceEvaluator::with_built_in_rules().unwrap();

    let assessments: Vec<CandidateAssessment> = candidates
        .iter()
        .map(|candidate| {
            let mut evidence = mass_evaluator.evaluate(&obs, candidate).unwrap();
            evidence.extend(fragment_evaluator.evaluate(&obs, candidate).unwrap());
            CandidateAssessment::new(candidate.id.clone(), EvidenceSet::new(evidence))
        })
        .collect();

    let ranker = Ranker::new();
    let ranked = ranker.rank(assessments);

    let order: Vec<&str> = ranked.iter().map(|a| a.candidate_id.as_str()).collect();
    assert_eq!(
        order,
        vec!["8-oxo-dG", "adenine-isomer", "mass-close-decoy"],
        "expected correct candidate first, under-evidenced isomer second, wrong-mass decoy last"
    );

    let top = &ranked[0];
    assert!(top.ranking_score.unwrap() > 0.0);
    assert!(
        ranked[2].ranking_score.unwrap() < 0.0,
        "wrong-mass decoy should be net-contradicted"
    );

    let explanation = explain(top);
    let text = explanation.to_text();
    assert!(text.contains("8-oxo-dG"));
    // At least one supporting (+) and the report should be non-trivial.
    assert!(text.contains('+'));
    assert!(explanation.lines.len() >= 5); // mass + precursor + neutral-loss + 2 CO-loss fragments

    // Structured explanation must also round-trip through JSON — it's a
    // first-class serializable representation (§11), not just text.
    let json = serde_json::to_string_pretty(&explanation).unwrap();
    let back: adductra::Explanation = serde_json::from_str(&json).unwrap();
    assert_eq!(explanation, back);

    // The top-level AdductReport (observation + ranked assessments +
    // provenance) must also be constructible and round-trip through JSON.
    let report = AdductReport {
        observation_id: obs.id.clone(),
        assessments: ranked,
        provenance: Provenance::derived("eight_oxo_dg_benchmark"),
    };
    let report_json = serde_json::to_string(&report).unwrap();
    let report_back: AdductReport = serde_json::from_str(&report_json).unwrap();
    assert_eq!(report, report_back);
    assert_eq!(report.assessments[0].candidate_id, "8-oxo-dG");
}

#[test]
fn missing_ms2_data_still_ranks_on_mass_alone_without_false_contradiction() {
    // A precursor-only observation (no MS2) must not manufacture
    // contradicting fragment evidence — absence of evidence is not
    // evidence of absence (§25).
    let obs =
        Observation::new("obs-precursor-only", 284.0989, 1, IonAdductType::ProtonAdd).unwrap();
    let candidate = AdductCandidate::from_formula(
        "8-oxo-dG",
        "8-oxo-2'-deoxyguanosine",
        "C10H13N5O5",
        Provenance::derived("benchmark-fixture"),
    )
    .unwrap()
    .with_nucleobase_origin(NucleobaseOrigin::Guanine);

    let mass_evaluator = MassEvidenceEvaluator::new(10.0).unwrap();
    let fragment_evaluator = FragmentEvidenceEvaluator::with_built_in_rules().unwrap();
    let mut evidence = mass_evaluator.evaluate(&obs, &candidate).unwrap();
    evidence.extend(fragment_evaluator.evaluate(&obs, &candidate).unwrap());

    let assessment = CandidateAssessment::new(candidate.id.clone(), EvidenceSet::new(evidence));
    let ranker = Ranker::new();
    let ranked = ranker.rank(vec![assessment]);

    // Mass + precursor consistency still support strongly; fragment rules
    // are all `Missing`, contributing zero rather than a penalty.
    assert!(ranked[0].ranking_score.unwrap() > 0.0);
    let explanation = explain(&ranked[0]);
    assert!(explanation.lines.iter().any(|l| l.text.contains("missing")));
}
