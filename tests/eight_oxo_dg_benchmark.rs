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
    AdductCandidate, AdductReport, CandidateAssessment, CandidateGenerator, EvidenceDetail,
    EvidenceDirection, EvidenceEvaluator, EvidenceKind, EvidenceSet, EvidenceSource,
    FragmentEvidenceEvaluator, IonAdductType, IsotopeEvidenceEvaluator, MassEvidenceEvaluator,
    NucleobaseOrigin, Observation, ProductIon, Provenance, Ranker, ReferencePeak,
    ReferenceSpectrum, SpectralLibraryEvidenceEvaluator, UserSuppliedGenerator, explain,
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
    .with_nucleobase_origin(NucleobaseOrigin::Other("8-oxo-guanine".to_string()));
    // Note on tagging: the CO-loss rules target `Other("8-oxo-guanine")`,
    // not the generic `NucleobaseOrigin::Guanine`, specifically so they
    // don't also fire on other guanine-derived adducts that aren't
    // 8-oxo-modified (e.g. AFB1-N7-Gua, tests/afb1_n7_gua_benchmark.rs) —
    // found by adding that second fixture and seeing the CO-loss rules
    // incorrectly match it under the old, broader targeting.

    // Same formula/mass as the correct candidate (an isomer), but tagged
    // with a different nucleobase origin — the 8-oxo-specific CO-loss
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
    let isotope_evaluator = IsotopeEvidenceEvaluator::new(0.005).unwrap();

    let assessments: Vec<CandidateAssessment> = candidates
        .iter()
        .map(|candidate| {
            let mut evidence = mass_evaluator.evaluate(&obs, candidate).unwrap();
            evidence.extend(fragment_evaluator.evaluate(&obs, candidate).unwrap());
            // No isotope label was used in this observation, so this
            // contributes one NotApplicable item (score-neutral) — run
            // it anyway so the full evaluator suite is exercised
            // end-to-end, not just the two that happen to score.
            evidence.extend(isotope_evaluator.evaluate(&obs, candidate).unwrap());
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
    assert!(explanation.lines.len() >= 6); // mass + precursor + neutral-loss + 2 CO-loss fragments + isotope (N/A)

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
    .with_nucleobase_origin(NucleobaseOrigin::Other("8-oxo-guanine".to_string()));

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

#[test]
fn present_but_wrong_fragment_peaks_lower_the_ranking_score() {
    // Same candidate, same correct precursor mass — but MS2 was acquired
    // and shows peaks at the wrong positions (contamination/misassignment)
    // rather than no MS2 data at all. Per §25 this must register as
    // Contradicting evidence, not Missing, and net-lower the score
    // relative to a spectrum that actually matches the expected pattern.
    let candidate = AdductCandidate::from_formula(
        "8-oxo-dG",
        "8-oxo-2'-deoxyguanosine",
        "C10H13N5O5",
        Provenance::derived("benchmark-fixture"),
    )
    .unwrap()
    .with_nucleobase_origin(NucleobaseOrigin::Other("8-oxo-guanine".to_string()));

    let matching_obs = observation();
    let wrong_peaks_obs =
        Observation::new("obs-wrong-peaks", 284.0989, 1, IonAdductType::ProtonAdd)
            .unwrap()
            .with_product_ions(vec![
                ProductIon::new(200.0, Some(50.0)).unwrap(),
                ProductIon::new(90.0, Some(20.0)).unwrap(),
            ]);

    let mass_evaluator = MassEvidenceEvaluator::new(10.0).unwrap();
    let fragment_evaluator = FragmentEvidenceEvaluator::with_built_in_rules().unwrap();

    let score_for = |obs: &Observation| -> f64 {
        let mut evidence = mass_evaluator.evaluate(obs, &candidate).unwrap();
        evidence.extend(fragment_evaluator.evaluate(obs, &candidate).unwrap());
        let assessment = CandidateAssessment::new(candidate.id.clone(), EvidenceSet::new(evidence));
        Ranker::new().rank(vec![assessment])[0]
            .ranking_score
            .unwrap()
    };

    let matching_score = score_for(&matching_obs);
    let wrong_score = score_for(&wrong_peaks_obs);
    assert!(
        matching_score > wrong_score,
        "matching={matching_score}, wrong={wrong_score}"
    );

    let mut evidence = mass_evaluator
        .evaluate(&wrong_peaks_obs, &candidate)
        .unwrap();
    evidence.extend(
        fragment_evaluator
            .evaluate(&wrong_peaks_obs, &candidate)
            .unwrap(),
    );
    let fragment_evidence: Vec<_> = evidence
        .iter()
        .filter(|e| {
            *e.kind() == EvidenceKind::DiagnosticFragment || *e.kind() == EvidenceKind::NeutralLoss
        })
        .collect();
    assert!(!fragment_evidence.is_empty());
    assert!(
        fragment_evidence
            .iter()
            .all(|e| e.direction() == EvidenceDirection::Contradicting),
        "{fragment_evidence:#?}"
    );
}

#[test]
fn spectral_library_match_uses_the_same_verified_8_oxo_dg_peaks() {
    // No new external data: the reference spectrum here is built from
    // the exact same real, cited product-ion triplet `observation()`
    // already uses (168.0516/140.0567/112.0618, intensities 100/40/15),
    // demonstrating the new evidence type end-to-end against data this
    // fixture has already independently verified.
    let candidate = AdductCandidate::from_formula(
        "8-oxo-dG",
        "8-oxo-2'-deoxyguanosine",
        "C10H13N5O5",
        Provenance::derived("benchmark-fixture"),
    )
    .unwrap()
    .with_nucleobase_origin(NucleobaseOrigin::Other("8-oxo-guanine".to_string()));

    let reference_peaks = vec![
        ReferencePeak::new(168.0516, 100.0).unwrap(),
        ReferencePeak::new(140.0567, 40.0).unwrap(),
        ReferencePeak::new(112.0618, 15.0).unwrap(),
    ];
    let reference = ReferenceSpectrum::new(
        candidate.id.clone(),
        reference_peaks,
        EvidenceSource::Experimental,
        "1.0.0",
    )
    .unwrap()
    .with_citation("see docs/landscape.md sec 5 (8-oxo-dG reference case)")
    .with_collision_energy("35 eV HCD");
    let evaluator = SpectralLibraryEvidenceEvaluator::new(vec![reference], 0.01, 0.7).unwrap();

    // Matching spectrum: the same triplet observation() already uses.
    let matching_evidence = evaluator.evaluate(&observation(), &candidate).unwrap();
    assert_eq!(matching_evidence.len(), 1);
    assert_eq!(
        matching_evidence[0].direction(),
        EvidenceDirection::Supporting
    );
    assert_eq!(
        *matching_evidence[0].kind(),
        EvidenceKind::SpectralLibraryMatch
    );
    if let EvidenceDetail::SpectralLibraryMatch {
        cosine_similarity, ..
    } = matching_evidence[0].detail()
    {
        assert!(
            (cosine_similarity.unwrap().get() - 1.0).abs() < 1e-9,
            "{:?}",
            cosine_similarity
        );
    }
    // Collision energy provenance flows through to the emitted evidence.
    assert_eq!(
        matching_evidence[0]
            .provenance()
            .parameters
            .get("collision_energy")
            .map(String::as_str),
        Some("35 eV HCD")
    );

    // Clearly different spectrum: same wrong-peaks pattern already used
    // to test FragmentEvidenceEvaluator's Contradicting behavior above.
    let wrong_obs = Observation::new("obs-wrong-peaks", 284.0989, 1, IonAdductType::ProtonAdd)
        .unwrap()
        .with_product_ions(vec![
            ProductIon::new(200.0, Some(50.0)).unwrap(),
            ProductIon::new(90.0, Some(20.0)).unwrap(),
        ]);
    let wrong_evidence = evaluator.evaluate(&wrong_obs, &candidate).unwrap();
    assert_eq!(
        wrong_evidence[0].direction(),
        EvidenceDirection::Contradicting
    );
}

#[test]
fn spectral_library_match_cross_validated_against_real_la_barbera_8_oxo_dg_spectrum() {
    // Independent cross-validation (`ROADMAP.md` v0.2.2): unlike the
    // test above, these peaks are NOT copied from `observation()` -- they
    // are real experimental values (40 eV collision energy) from La
    // Barbera G, Nommesen KD, Cuparencu C, Stanstrup J, Dragsted LO
    // (2022), "A Comprehensive Database for DNA Adductomics," Frontiers
    // in Chemistry 10:908572, doi:10.3389/fchem.2022.908572 (CC BY 4.0;
    // `gitlab.com/nexs-metabolomics/projects/dna_adductomics_database`,
    // `_input/MS MS spectra standards.xlsx`, `8-oxo-dG` sheet). They
    // land within ~0.0005 Da of this fixture's own independently-derived
    // 168.0516/140.0567/112.0618 triplet -- a real, external source
    // corroborating already-shipped fixture data, not just a second
    // synthetic test of the same numbers.
    let candidate = AdductCandidate::from_formula(
        "8-oxo-dG",
        "8-oxo-2'-deoxyguanosine",
        "C10H13N5O5",
        Provenance::derived("benchmark-fixture"),
    )
    .unwrap()
    .with_nucleobase_origin(NucleobaseOrigin::Other("8-oxo-guanine".to_string()));

    let reference_peaks = vec![
        ReferencePeak::new(168.05154, 100.0).unwrap(),
        ReferencePeak::new(140.05707, 63.69).unwrap(),
        ReferencePeak::new(112.06223, 23.18).unwrap(),
    ];
    let reference = ReferenceSpectrum::new(
        candidate.id.clone(),
        reference_peaks,
        EvidenceSource::Experimental,
        "1.0.0",
    )
    .unwrap()
    .with_citation(
        "La Barbera et al. 2022, Frontiers in Chemistry 10:908572, doi:10.3389/fchem.2022.908572 \
         (gitlab.com/nexs-metabolomics/projects/dna_adductomics_database, CC BY 4.0)",
    )
    .with_collision_energy("40 eV");
    let evaluator = SpectralLibraryEvidenceEvaluator::new(vec![reference], 0.01, 0.7).unwrap();

    let evidence = evaluator.evaluate(&observation(), &candidate).unwrap();
    assert_eq!(evidence.len(), 1);
    assert_eq!(evidence[0].direction(), EvidenceDirection::Supporting);
    assert!(matches!(
        evidence[0].detail(),
        EvidenceDetail::SpectralLibraryMatch { .. }
    ));
    if let EvidenceDetail::SpectralLibraryMatch {
        cosine_similarity, ..
    } = evidence[0].detail()
    {
        let cosine = cosine_similarity.unwrap().get();
        assert!(cosine > 0.98, "expected high cosine, got {cosine}");
    }
}
