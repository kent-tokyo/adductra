//! Fourth benchmark reference case (`ROADMAP.md` v0.2.2): the first case
//! sourced from an external database rather than a single paper — La
//! Barbera G, Nommesen KD, Cuparencu C, Stanstrup J, Dragsted LO (2022),
//! "A Comprehensive Database for DNA Adductomics," Frontiers in
//! Chemistry 10:908572, doi:10.3389/fchem.2022.908572. Published under
//! Frontiers' CC BY 4.0 policy; the paper's Data Availability Statement
//! names the GitLab repository (`gitlab.com/nexs-metabolomics/projects/
//! dna_adductomics_database`) — no separate LICENSE file exists there,
//! this is the same CC BY 4.0 basis `ROADMAP.md` already recorded.
//!
//! O6-methyl-2'-deoxyguanosine (O6-Me-dG), a simple alkylation adduct
//! (DOI:10.1017/S0007114515001750, InChIKey BCKDNMPYCIOBTA-FSDSQADBSA-N
//! in the database's master compound table).
//!
//! All masses independently hand-computed from the formula against
//! `mass_table`'s constants, not copied from the database:
//! - O6-Me-dG C11H15N5O4, neutral mass 281.112404, [M+H]+ = 282.119680
//! - O6-methylguanine free base C6H7N5O (deoxyribose loss), [M+H]+ =
//!   166.072336
//! - delta = 116.047344, matching the existing `nucleoside-deoxyribose-
//!   loss` rule's 116.0473 to 4 significant figures — the same
//!   nucleobase-agnostic rule `tests/ethenoadenine_benchmark.rs` already
//!   confirmed generalizes; this fixture adds **zero new rule data**.
//!
//! Note: the database's own master table reports a "charged monoisotopic
//! mass" of 282.120230 (166.07293 for the -dR fragment) — about 0.00055
//! Da higher than the value used here. That's the mass of a neutral H
//! atom (1.007825) rather than a proton (1.007276), a common
//! simplification; `mass_table.rs` deliberately uses the proton mass
//! (its own doc comment: "using the H-atom mass instead omits the
//! electron and is wrong"), so this fixture follows the crate's own
//! established convention rather than the database's reported value.
//!
//! The diagnostic fragment's real observed value (166.07292 at 40 eV,
//! from `_input/MS MS spectra standards.xlsx`'s `O6-Me-dG` sheet, the
//! dominant peak at that collision energy) is used for the spectral-
//! library-match `ReferenceSpectrum` below, distinct from the
//! theoretical value used for the `Observation` — see that test for the
//! real-data-vs-theory cross-check. The sheet's second-strongest peak
//! (149.04586, presumably a secondary NH3 loss) is deliberately left
//! out: using it would mean asserting a specific fragmentation
//! mechanism this fixture hasn't independently verified, which the
//! round's design intentionally avoids (`ROADMAP.md` v0.2.2 — zero new
//! fragment-mechanism claims, only the pre-existing generic rule).
//!
//! Decoy: 1-methyl-dG (1-Me-dG, DOI:10.1016/j.jpba.2019.01.034,
//! InChIKey VJSHSPNFYFRXGN-FSDSQADBSA-N) — a real, distinct N1-methylated
//! regioisomer from the same database, not a fabricated decoy. Exact
//! same formula/mass as O6-Me-dG, which is deliberately a *harder* case
//! than 8-oxo-dG-vs-adenine-isomer, not the same one: 8-oxo-dG's CO-loss
//! rules are narrowly `NucleobaseOrigin`-targeted and so discriminate
//! its isomer decoy, but `nucleoside-deoxyribose-loss` is generic
//! (`Any`-targeted, computed from the observed delta, blind to candidate
//! structure) and fires identically for both O6-Me-dG and this decoy —
//! confirmed as a genuine tie below, not glossed over. The source
//! database has no reference spectrum for this decoy either, so this
//! fixture also documents a real, current gap: distinguishing same-
//! formula regioisomers needs a per-isomer reference spectrum this round
//! doesn't have for every isomer (see `ROADMAP.md` v0.2.2).

use adductra::{
    AdductCandidate, CandidateAssessment, EvidenceDetail, EvidenceDirection, EvidenceEvaluator,
    EvidenceKind, EvidenceSet, EvidenceSource, FragmentEvidenceEvaluator, IonAdductType,
    MassEvidenceEvaluator, Observation, ProductIon, Provenance, Ranker, ReferencePeak,
    ReferenceSpectrum, SpectralLibraryEvidenceEvaluator,
};

fn observation() -> Observation {
    Observation::new("obs-o6medg-1", 282.119680, 1, IonAdductType::ProtonAdd)
        .unwrap()
        .with_product_ions(vec![ProductIon::new(166.072336, Some(100.0)).unwrap()])
}

fn correct_candidate() -> AdductCandidate {
    AdductCandidate::from_formula(
        "O6-Me-dG",
        "O6-methyl-2'-deoxyguanosine",
        "C11H15N5O4",
        Provenance::derived("benchmark-fixture"),
    )
    .unwrap()
}

fn decoy_candidate() -> AdductCandidate {
    AdductCandidate::from_formula(
        "1-Me-dG",
        "1-methyl-2'-deoxyguanosine (real, same formula/mass, wrong regioisomer)",
        "C11H15N5O4",
        Provenance::derived("benchmark-fixture"),
    )
    .unwrap()
}

#[test]
fn o6_me_dg_and_1_me_dg_decoy_tie_on_mass_and_generic_rule_evidence_alone() {
    // A genuine, documented limitation (`ROADMAP.md` v0.2.2), not a
    // fabricated edge case: O6-Me-dG and its N1-methylated regioisomer
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
fn spectral_library_match_against_real_la_barbera_o6_me_dg_spectrum() {
    // Real experimental peaks from the database's own standards workbook
    // (40 eV collision energy), not the theoretical values `observation()`
    // uses -- a genuine independent cross-check, not a self-referential
    // one.
    let reference_peaks = vec![ReferencePeak::new(166.07292, 100.0).unwrap()];
    let reference = ReferenceSpectrum::new(
        "O6-Me-dG",
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

    let evidence = evaluator
        .evaluate(&observation(), &correct_candidate())
        .unwrap();
    assert_eq!(evidence.len(), 1);
    assert_eq!(evidence[0].direction(), EvidenceDirection::Supporting);
    assert_eq!(*evidence[0].kind(), EvidenceKind::SpectralLibraryMatch);
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
