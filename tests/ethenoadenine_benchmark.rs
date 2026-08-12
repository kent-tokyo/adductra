//! Third benchmark reference case (`ROADMAP.md` Phase 6): the first case
//! built on a nucleobase other than guanine, specifically to test
//! whether the evidence engine generalizes rather than happening to work
//! for guanine chemistry.
//!
//! 1,N6-ethenoadenine (εdA) — an exocyclic adenine adduct formed by vinyl
//! chloride exposure and endogenous lipid peroxidation (4-HNE). Unlike
//! the two existing cases, this fixture adds **zero new rules**: the
//! etheno bridge is fused entirely into the base, so the sugar/
//! glycosidic bond is untouched and the existing nucleobase-agnostic
//! `nucleoside-deoxyribose-loss` rule (`rules/dna_adduct_fragments.json`,
//! target `Any`) already covers its one diagnostic transition. That's
//! the generalization test: the rule works correctly for an unrelated
//! nucleobase family without any adenine-specific code or data.
//!
//! All masses independently hand-computed from formulas against
//! `mass_table`'s constants (not copied from papers) and cross-checked
//! to match the literature's reported nominal m/z (276, 160, 281):
//! - εdA nucleoside C12H13N5O3, [M+H]+ = 276.109116
//! - εdA free base C7H5N5, [M+H]+ = 160.061772 (base-loss fragment)
//! - delta = 116.047344, matching the existing rule's 116.0473 to 4
//!   significant figures — an independent confirmation the rule's value
//!   is correct, not just correct for 8-oxo-dG.
//! - [15N5]-εdA internal standard: 281.094290 (paper reports 281)
//!
//! Source: Cui S, Li H, Wang S, Jiang X, Zhang S, Zhang R, Sun X.
//! "Ultrasensitive UPLC-MS-MS Method for the Quantitation of Etheno-DNA
//! Adducts in Human Urine." Int J Environ Res Public Health 2014,
//! 11(10):10902-10914. doi:10.3390/ijerph111010902 (open text:
//! PMC4211013). Positive ESI, [M+H]+, 15N5-labeled internal standard
//! (all 5 labeled nitrogens are adenine-ring nitrogens, none on the
//! sugar — confirmed by the paper's own +5 Da shift on both precursor
//! and product ion). Formula cross-checked against PubChem CID 4250940
//! (nucleoside) and CID 104994 (free base).
//!
//! Decoy: 1,N2-etheno-2'-deoxyguanosine (εdG, C12H13N5O4, CAS
//! 108929-11-9) — a real, distinct, commonly co-measured etheno adduct
//! formed by the same exposure chemistry, not a fabricated decoy. Its
//! mass genuinely differs from εdA's (one more oxygen), so it tests mass
//! rejection rather than same-mass/wrong-origin discrimination (that
//! axis is already covered by the other two fixtures).

use adductra::mass_table::Element;
use adductra::{
    AdductCandidate, CandidateAssessment, EvidenceDirection, EvidenceEvaluator, EvidenceSet,
    FragmentEvidenceEvaluator, IonAdductType, IsotopeEvidenceEvaluator, IsotopeLabel,
    MassEvidenceEvaluator, NucleobaseOrigin, Observation, ProductIon, Provenance, Ranker,
};

fn eda_observation() -> Observation {
    Observation::new("obs-eda-1", 276.109116, 1, IonAdductType::ProtonAdd)
        .unwrap()
        .with_product_ions(vec![ProductIon::new(160.061772, Some(100.0)).unwrap()])
}

fn eda_candidate() -> AdductCandidate {
    AdductCandidate::from_formula(
        "etheno-dA",
        "1,N6-etheno-2'-deoxyadenosine",
        "C12H13N5O3",
        Provenance::derived("benchmark-fixture"),
    )
    .unwrap()
    .with_nucleobase_origin(NucleobaseOrigin::Adenine)
}

fn edg_decoy_candidate() -> AdductCandidate {
    AdductCandidate::from_formula(
        "etheno-dG",
        "1,N2-etheno-2'-deoxyguanosine (real, co-formed, wrong mass for this spectrum)",
        "C12H13N5O4",
        Provenance::derived("benchmark-fixture"),
    )
    .unwrap()
    .with_nucleobase_origin(NucleobaseOrigin::Guanine)
}

#[test]
fn etheno_da_outranks_etheno_dg_decoy_using_only_pre_existing_rules() {
    let obs = eda_observation();
    let correct = eda_candidate();
    let decoy = edg_decoy_candidate();

    let mass_evaluator = MassEvidenceEvaluator::new(10.0).unwrap();
    let fragment_evaluator = FragmentEvidenceEvaluator::with_built_in_rules().unwrap();

    let assess = |candidate: &AdductCandidate| -> CandidateAssessment {
        let mut evidence = mass_evaluator.evaluate(&obs, candidate).unwrap();
        evidence.extend(fragment_evaluator.evaluate(&obs, candidate).unwrap());
        CandidateAssessment::new(candidate.id.clone(), EvidenceSet::new(evidence))
    };

    let ranked = Ranker::new().rank(vec![assess(&correct), assess(&decoy)]);
    assert_eq!(ranked[0].candidate_id, "etheno-dA");
    assert_eq!(ranked[1].candidate_id, "etheno-dG");
    assert!(ranked[0].ranking_score.unwrap() > 0.0);
    assert!(
        ranked[1].ranking_score.unwrap() < ranked[0].ranking_score.unwrap(),
        "εdG (~16 Da off this spectrum) must rank below εdA"
    );

    // Exactly one fragment rule fires (the pre-existing generic
    // deoxyribose-loss rule) -- no adenine-specific rule exists or was
    // added for this fixture.
    let eda_fragments = fragment_evaluator.evaluate(&obs, &correct).unwrap();
    assert_eq!(eda_fragments.len(), 1);
    assert_eq!(eda_fragments[0].direction(), EvidenceDirection::Supporting);
    assert!(
        eda_fragments[0]
            .method()
            .starts_with("rule:nucleoside-deoxyribose-loss")
    );
}

#[test]
fn fifteen_n5_labeled_etheno_da_shift_supported_by_isotope_evidence() {
    // Mirrors the AFB1-N7-Gua isotope test: adenine's ring also has
    // exactly 5 nitrogens, all retained in this labeling scheme, so a
    // 6th labeled nitrogen is equally chemically impossible here.
    let unlabeled_mz = 276.109116;
    let label = IsotopeLabel::new(Element::N, 15, 5);
    let shift = label.total_shift_da().unwrap();
    let labeled_mz = unlabeled_mz + shift;
    assert!((labeled_mz - 281.094290).abs() < 1e-3, "got {labeled_mz}");

    let obs = Observation::new("obs-eda-15n5", labeled_mz, 1, IonAdductType::ProtonAdd)
        .unwrap()
        .with_isotope_labels(vec![label]);

    let isotope_evaluator = IsotopeEvidenceEvaluator::new(0.005).unwrap();
    let evidence = isotope_evaluator.evaluate(&obs, &eda_candidate()).unwrap();
    assert_eq!(evidence.len(), 1);
    assert_eq!(evidence[0].direction(), EvidenceDirection::Supporting);

    let mut bad_obs = obs.clone();
    bad_obs.isotope_labels = vec![IsotopeLabel::new(Element::N, 15, 6)];
    let result = isotope_evaluator.evaluate(&bad_obs, &eda_candidate());
    assert!(result.is_err());
}
