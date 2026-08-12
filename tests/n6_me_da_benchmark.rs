//! Fifth benchmark reference case (`ROADMAP.md` v0.2.2), sourced from the
//! same La Barbera et al. 2022 database as `tests/o6_me_dg_benchmark.rs`
//! (Frontiers in Chemistry 10:908572, doi:10.3389/fchem.2022.908572, CC
//! BY 4.0) -- see that file's module doc for the full licensing basis.
//!
//! N6-methyl-2'-deoxyadenosine (N6-Me-dA), a simple alkylation adduct
//! (DOI:10.1016/j.envint.2018.10.041, InChIKey
//! DYSDOYRQWBDGQQ-BWZBUEFSSA-N in the database's master compound table).
//!
//! All masses independently hand-computed from the formula against
//! `mass_table`'s constants, not copied from the database:
//! - N6-Me-dA C11H15N5O3, neutral mass 265.117489, [M+H]+ = 266.124766
//! - N6-methyladenine free base C6H7N5 (deoxyribose loss), [M+H]+ =
//!   150.077422
//! - delta = 116.047344, matching the pre-existing nucleobase-agnostic
//!   `nucleoside-deoxyribose-loss` rule's 116.0473 to 4 significant
//!   figures -- the same value independently confirmed by
//!   `tests/ethenoadenine_benchmark.rs` and `tests/o6_me_dg_benchmark.rs`.
//!   Zero new rule data added.
//!
//! The diagnostic fragment's real observed value (150.0775 at 20 eV,
//! from the database's `MS MS spectra standards.xlsx`, `N-Me-dA` sheet)
//! is used for the spectral-library-match `ReferenceSpectrum` below,
//! distinct from the theoretical value used for the `Observation`.
//!
//! Decoy: 2-methyl-dA (2-Me-dA, DOI:10.1016/j.jpba.2019.01.034, InChIKey
//! KSUGGAKGOCLWPT-BWZBUEFSSA-N) -- a real, distinct N2-methylated
//! regioisomer, exact same formula/mass as N6-Me-dA. `nucleoside-
//! deoxyribose-loss` is `Any`-targeted and computed purely from the
//! observed delta, blind to candidate structure, so it fires
//! identically for both -- confirmed as a genuine tie below (see
//! `tests/o6_me_dg_benchmark.rs`'s module doc for the full reasoning).
//! The source database has no reference spectrum for this decoy either,
//! documenting a real, current gap (`ROADMAP.md` v0.2.2).

use adductra::{
    AdductCandidate, CandidateAssessment, EvidenceDetail, EvidenceDirection, EvidenceEvaluator,
    EvidenceSet, EvidenceSource, FragmentEvidenceEvaluator, IonAdductType, MassEvidenceEvaluator,
    Observation, ProductIon, Provenance, Ranker, ReferencePeak, ReferenceSpectrum,
    SpectralLibraryEvidenceEvaluator,
};

fn observation() -> Observation {
    Observation::new("obs-n6medga-1", 266.124766, 1, IonAdductType::ProtonAdd)
        .unwrap()
        .with_product_ions(vec![ProductIon::new(150.077422, Some(100.0)).unwrap()])
}

fn correct_candidate() -> AdductCandidate {
    AdductCandidate::from_formula(
        "N6-Me-dA",
        "N6-methyl-2'-deoxyadenosine",
        "C11H15N5O3",
        Provenance::derived("benchmark-fixture"),
    )
    .unwrap()
}

fn decoy_candidate() -> AdductCandidate {
    AdductCandidate::from_formula(
        "2-Me-dA",
        "2-methyl-2'-deoxyadenosine (real, same formula/mass, wrong regioisomer)",
        "C11H15N5O3",
        Provenance::derived("benchmark-fixture"),
    )
    .unwrap()
}

#[test]
fn n6_me_da_and_2_me_da_decoy_tie_on_mass_and_generic_rule_evidence_alone() {
    // A genuine, documented limitation (`ROADMAP.md` v0.2.2), not a
    // fabricated edge case: N6-Me-dA and its N2-methylated regioisomer
    // share the exact same formula, so mass evidence can't tell them
    // apart, and `nucleoside-deoxyribose-loss` is computed purely from
    // the observed precursor/fragment delta -- it doesn't inspect
    // candidate structure, so it fires identically for both. Same-
    // formula regioisomers are only distinguishable with a real,
    // per-candidate reference spectrum (see the spectral-library-match
    // test below); no such spectrum exists in the source database for
    // this decoy specifically, so this evidence subset alone cannot
    // discriminate them -- and this test asserts that honestly instead
    // of hiding it.
    let obs = observation();
    let correct = correct_candidate();
    let decoy = decoy_candidate();

    let mass_evaluator = MassEvidenceEvaluator::new(10.0).unwrap();
    let fragment_evaluator = FragmentEvidenceEvaluator::with_built_in_rules().unwrap();

    let assess = |candidate: &AdductCandidate| -> CandidateAssessment {
        let mut evidence = mass_evaluator.evaluate(&obs, candidate).unwrap();
        evidence.extend(fragment_evaluator.evaluate(&obs, candidate).unwrap());
        CandidateAssessment::new(candidate.id.clone(), EvidenceSet::new(evidence))
    };

    let fragments = fragment_evaluator.evaluate(&obs, &correct).unwrap();
    assert_eq!(fragments.len(), 1);
    assert_eq!(fragments[0].direction(), EvidenceDirection::Supporting);
    assert!(
        fragments[0]
            .method()
            .starts_with("rule:nucleoside-deoxyribose-loss")
    );

    let ranked = Ranker::new().rank(vec![assess(&correct), assess(&decoy)]);
    assert!(ranked[0].ranking_score.unwrap() > 0.0);
    assert_eq!(
        ranked[0].ranking_score, ranked[1].ranking_score,
        "expected a genuine tie without a reference spectrum for the decoy"
    );
}

#[test]
fn spectral_library_match_against_real_la_barbera_n6_me_da_spectrum() {
    // Real experimental peak from the database's own standards workbook
    // (20 eV collision energy), not the theoretical value `observation()`
    // uses.
    let reference_peaks = vec![ReferencePeak::new(150.0775, 100.0).unwrap()];
    let reference = ReferenceSpectrum::new(
        "N6-Me-dA",
        reference_peaks,
        EvidenceSource::Experimental,
        "1.0.0",
    )
    .unwrap()
    .with_citation(
        "La Barbera et al. 2022, Frontiers in Chemistry 10:908572, doi:10.3389/fchem.2022.908572 \
         (gitlab.com/nexs-metabolomics/projects/dna_adductomics_database, CC BY 4.0)",
    )
    .with_collision_energy("20 eV");
    let evaluator = SpectralLibraryEvidenceEvaluator::new(vec![reference], 0.01, 0.7).unwrap();

    let evidence = evaluator
        .evaluate(&observation(), &correct_candidate())
        .unwrap();
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
        assert!(cosine > 0.99, "expected near-1.0 cosine, got {cosine}");
    }

    // This does NOT demonstrate telling the two regioisomers apart by
    // their differing spectra (the source database has no spectrum for
    // the decoy at all -- see the tie test above). It demonstrates a
    // narrower, still real property: the reference library is scoped
    // per candidate_id, so a candidate with no known reference spectrum
    // abstains (NotApplicable) instead of silently reusing another
    // candidate's match just because the formula happens to be the same.
    let decoy_evidence = evaluator
        .evaluate(&observation(), &decoy_candidate())
        .unwrap();
    assert_eq!(decoy_evidence.len(), 1);
    assert_eq!(
        decoy_evidence[0].direction(),
        EvidenceDirection::NotApplicable
    );
}
