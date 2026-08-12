//! `AGENTS.md` §15: benchmark corpus metrics (top-1 accuracy, top-k
//! recall, MRR, candidate reduction, ranking margin, evidence coverage)
//! computed across Adductra's reference cases.
//!
//! ponytail: metrics are plain Rust here, not `veridict` (the sibling
//! crate `docs/landscape.md` §3 earmarks for this). `veridict`'s real
//! API (win-rate/Elo/bootstrap-CI/SPRT regression gating, per that
//! doc's research pass) reads as built for statistically comparing two
//! ranking configurations across many trials — overkill, and not
//! confidently understood well enough to use correctly yet, for
//! computing straightforward formulas over 3 known-answer cases. Revisit
//! when there's an actual A vs. B ranking comparison to run.
//!
//! Fixture data intentionally duplicates (small, ~15-25 lines each) the
//! setup in `tests/eight_oxo_dg_benchmark.rs`, `tests/afb1_n7_gua_benchmark.rs`,
//! `tests/ethenoadenine_benchmark.rs`, `tests/o6_me_dg_benchmark.rs`,
//! `tests/n6_me_da_benchmark.rs`, and `tests/n2_ethyl_dg_benchmark.rs`
//! rather than importing from them — those files are exercising specific
//! evidence-type behaviors and already pass; this file adds a
//! corpus-level view without risking a refactor of already-verified
//! tests.

use adductra::{
    AdductCandidate, CandidateAssessment, EvidenceDirection, EvidenceEvaluator, EvidenceSet,
    FragmentEvidenceEvaluator, IonAdductType, IsotopeEvidenceEvaluator, MassEvidenceEvaluator,
    NucleobaseOrigin, Observation, ProductIon, Provenance, Ranker,
};

struct BenchmarkCase {
    name: &'static str,
    observation: Observation,
    candidates: Vec<AdductCandidate>,
    correct_candidate_id: &'static str,
}

fn eight_oxo_dg_case() -> BenchmarkCase {
    let observation = Observation::new("obs-8oxodg-1", 284.0989, 1, IonAdductType::ProtonAdd)
        .unwrap()
        .with_product_ions(vec![
            ProductIon::new(168.0516, Some(100.0)).unwrap(),
            ProductIon::new(140.0567, Some(40.0)).unwrap(),
            ProductIon::new(112.0618, Some(15.0)).unwrap(),
        ]);
    let candidates = vec![
        AdductCandidate::from_formula(
            "8-oxo-dG",
            "8-oxo-2'-deoxyguanosine",
            "C10H13N5O5",
            Provenance::derived("corpus"),
        )
        .unwrap()
        .with_nucleobase_origin(NucleobaseOrigin::Other("8-oxo-guanine".to_string())),
        AdductCandidate::from_formula(
            "adenine-isomer",
            "isomeric adenine-derived decoy",
            "C10H13N5O5",
            Provenance::derived("corpus"),
        )
        .unwrap()
        .with_nucleobase_origin(NucleobaseOrigin::Adenine),
        AdductCandidate::from_formula(
            "mass-close-decoy",
            "near-isobaric wrong-formula decoy",
            "C11H17N5O4",
            Provenance::derived("corpus"),
        )
        .unwrap(),
    ];
    BenchmarkCase {
        name: "8-oxo-dG",
        observation,
        candidates,
        correct_candidate_id: "8-oxo-dG",
    }
}

fn afb1_n7_gua_case() -> BenchmarkCase {
    let observation = Observation::new("obs-afb1-n7-gua", 480.114989, 1, IonAdductType::ProtonAdd)
        .unwrap()
        .with_product_ions(vec![
            ProductIon::new(152.056686, Some(80.0)).unwrap(),
            ProductIon::new(329.065579, Some(100.0)).unwrap(),
        ]);
    let candidates = vec![
        AdductCandidate::from_formula(
            "AFB1-N7-Gua",
            "aflatoxin B1 - N7-guanine adduct",
            "C22H17N5O8",
            Provenance::derived("corpus"),
        )
        .unwrap()
        .with_nucleobase_origin(NucleobaseOrigin::Guanine),
        AdductCandidate::from_formula(
            "AFB1-FapyGua",
            "aflatoxin B1 - formamidopyrimidine adduct",
            "C22H19N5O9",
            Provenance::derived("corpus"),
        )
        .unwrap()
        .with_nucleobase_origin(NucleobaseOrigin::Guanine),
    ];
    BenchmarkCase {
        name: "AFB1-N7-Gua",
        observation,
        candidates,
        correct_candidate_id: "AFB1-N7-Gua",
    }
}

fn etheno_da_case() -> BenchmarkCase {
    // Third reference case, and the first non-guanine one -- see
    // tests/ethenoadenine_benchmark.rs for full sourcing. Uses only the
    // pre-existing generic deoxyribose-loss rule (no adenine-specific
    // rule data), which is itself the point: this case exercises
    // generalization, not new rule coverage.
    let observation = Observation::new("obs-eda-1", 276.109116, 1, IonAdductType::ProtonAdd)
        .unwrap()
        .with_product_ions(vec![ProductIon::new(160.061772, Some(100.0)).unwrap()]);
    let candidates = vec![
        AdductCandidate::from_formula(
            "etheno-dA",
            "1,N6-etheno-2'-deoxyadenosine",
            "C12H13N5O3",
            Provenance::derived("corpus"),
        )
        .unwrap()
        .with_nucleobase_origin(NucleobaseOrigin::Adenine),
        AdductCandidate::from_formula(
            "etheno-dG",
            "1,N2-etheno-2'-deoxyguanosine (real, co-formed, wrong mass for this spectrum)",
            "C12H13N5O4",
            Provenance::derived("corpus"),
        )
        .unwrap()
        .with_nucleobase_origin(NucleobaseOrigin::Guanine),
    ];
    BenchmarkCase {
        name: "etheno-dA",
        observation,
        candidates,
        correct_candidate_id: "etheno-dA",
    }
}

fn o6_me_dg_case() -> BenchmarkCase {
    // Fourth reference case (`ROADMAP.md` v0.2.2), sourced from La
    // Barbera et al. 2022 -- see tests/o6_me_dg_benchmark.rs for the
    // full citation trail and independent mass derivation. Uses only the
    // pre-existing generic deoxyribose-loss rule.
    let observation = Observation::new("obs-o6medg-1", 282.119680, 1, IonAdductType::ProtonAdd)
        .unwrap()
        .with_product_ions(vec![ProductIon::new(166.072336, Some(100.0)).unwrap()]);
    let candidates = vec![
        AdductCandidate::from_formula(
            "O6-Me-dG",
            "O6-methyl-2'-deoxyguanosine",
            "C11H15N5O4",
            Provenance::derived("corpus"),
        )
        .unwrap(),
        AdductCandidate::from_formula(
            "1-Me-dG",
            "1-methyl-2'-deoxyguanosine (real, same formula/mass, wrong regioisomer)",
            "C11H15N5O4",
            Provenance::derived("corpus"),
        )
        .unwrap(),
    ];
    BenchmarkCase {
        name: "O6-Me-dG",
        observation,
        candidates,
        correct_candidate_id: "O6-Me-dG",
    }
}

fn n6_me_da_case() -> BenchmarkCase {
    // Fifth reference case (`ROADMAP.md` v0.2.2) -- see
    // tests/n6_me_da_benchmark.rs for the full citation trail.
    let observation = Observation::new("obs-n6medga-1", 266.124766, 1, IonAdductType::ProtonAdd)
        .unwrap()
        .with_product_ions(vec![ProductIon::new(150.077422, Some(100.0)).unwrap()]);
    let candidates = vec![
        AdductCandidate::from_formula(
            "N6-Me-dA",
            "N6-methyl-2'-deoxyadenosine",
            "C11H15N5O3",
            Provenance::derived("corpus"),
        )
        .unwrap(),
        AdductCandidate::from_formula(
            "2-Me-dA",
            "2-methyl-2'-deoxyadenosine (real, same formula/mass, wrong regioisomer)",
            "C11H15N5O3",
            Provenance::derived("corpus"),
        )
        .unwrap(),
    ];
    BenchmarkCase {
        name: "N6-Me-dA",
        observation,
        candidates,
        correct_candidate_id: "N6-Me-dA",
    }
}

fn n2_ethyl_dg_case() -> BenchmarkCase {
    // Sixth reference case (`ROADMAP.md` v0.2.2) -- see
    // tests/n2_ethyl_dg_benchmark.rs for the full citation trail.
    let observation = Observation::new("obs-n2etdg-1", 296.135331, 1, IonAdductType::ProtonAdd)
        .unwrap()
        .with_product_ions(vec![ProductIon::new(180.087986, Some(100.0)).unwrap()]);
    let candidates = vec![
        AdductCandidate::from_formula(
            "N2-Ethyl-dG",
            "N2-ethyl-2'-deoxyguanosine",
            "C12H17N5O4",
            Provenance::derived("corpus"),
        )
        .unwrap(),
        AdductCandidate::from_formula(
            "O6-Ethyl-dG",
            "O6-ethyl-2'-deoxyguanosine (real, same formula/mass, wrong regioisomer)",
            "C12H17N5O4",
            Provenance::derived("corpus"),
        )
        .unwrap(),
    ];
    BenchmarkCase {
        name: "N2-Ethyl-dG",
        observation,
        candidates,
        correct_candidate_id: "N2-Ethyl-dG",
    }
}

fn rank_case(case: &BenchmarkCase) -> Vec<CandidateAssessment> {
    let mass_evaluator = MassEvidenceEvaluator::new(10.0).unwrap();
    let fragment_evaluator = FragmentEvidenceEvaluator::with_built_in_rules().unwrap();
    let isotope_evaluator = IsotopeEvidenceEvaluator::new(0.005).unwrap();

    let assessments: Vec<CandidateAssessment> = case
        .candidates
        .iter()
        .map(|candidate| {
            let mut evidence = mass_evaluator
                .evaluate(&case.observation, candidate)
                .unwrap();
            evidence.extend(
                fragment_evaluator
                    .evaluate(&case.observation, candidate)
                    .unwrap(),
            );
            evidence.extend(
                isotope_evaluator
                    .evaluate(&case.observation, candidate)
                    .unwrap(),
            );
            CandidateAssessment::new(candidate.id.clone(), EvidenceSet::new(evidence))
        })
        .collect();

    Ranker::new().rank(assessments)
}

/// 1-indexed rank of the correct candidate in `ranked`.
fn correct_rank(ranked: &[CandidateAssessment], correct_id: &str) -> usize {
    ranked
        .iter()
        .position(|a| a.candidate_id == correct_id)
        .map(|i| i + 1)
        .expect("correct candidate must be present in its own case's candidate list")
}

/// Fraction of an evidence set that was actually evaluable (Supporting
/// or Contradicting), as opposed to Missing/Unavailable/NotApplicable.
fn evidence_coverage(assessment: &CandidateAssessment) -> f64 {
    let total = assessment.evidence.len();
    if total == 0 {
        return 0.0;
    }
    let evaluable = assessment
        .evidence
        .iter()
        .filter(|e| {
            matches!(
                e.direction(),
                EvidenceDirection::Supporting | EvidenceDirection::Contradicting
            )
        })
        .count();
    evaluable as f64 / total as f64
}

/// Reference cases whose decoy is only distinguishable by mass and the
/// generic `Any`-targeted `nucleoside-deoxyribose-loss` rule (no
/// candidate-specific rule, no per-decoy reference spectrum in the
/// source database) genuinely tie with their decoy under this evaluator
/// set. That's not a bug -- `nucleoside-deoxyribose-loss` is computed
/// purely from the observed precursor/fragment delta and doesn't
/// inspect candidate structure at all, so it cannot discriminate
/// same-formula regioisomers by design. See
/// `tests/o6_me_dg_benchmark.rs`'s module doc for the full reasoning and
/// the dedicated tie test. Distinguishing these requires a real,
/// per-candidate reference spectrum (`SpectralLibraryEvidenceEvaluator`,
/// not used in `rank_case` here since none of the three decoys below
/// have one in the source database -- adding a spectrum that only
/// covers the correct candidate would make this metric measure fixture
/// labeling, not the evidence engine's actual discriminating power).
const KNOWN_ISOMER_TIES: &[&str] = &["O6-Me-dG", "N6-Me-dA", "N2-Ethyl-dG"];

#[test]
fn corpus_metrics_meet_v01_baseline() {
    let cases = [
        eight_oxo_dg_case(),
        afb1_n7_gua_case(),
        etheno_da_case(),
        o6_me_dg_case(),
        n6_me_da_case(),
        n2_ethyl_dg_case(),
    ];
    let n = cases.len() as f64;

    let mut top_1_hits = 0usize;
    let mut top_2_hits = 0usize;
    let mut reciprocal_ranks = Vec::new();
    let mut margins = Vec::new();
    let mut coverages = Vec::new();
    let mut total_candidates = 0usize;
    let mut net_excluded = 0usize;

    for case in &cases {
        let ranked = rank_case(case);
        let rank = correct_rank(&ranked, case.correct_candidate_id);
        let is_known_tie = KNOWN_ISOMER_TIES.contains(&case.correct_candidate_id);

        if rank == 1 {
            top_1_hits += 1;
        }
        if rank <= 2 {
            top_2_hits += 1;
        }
        reciprocal_ranks.push(1.0 / rank as f64);

        let correct = ranked
            .iter()
            .find(|a| a.candidate_id == case.correct_candidate_id)
            .unwrap();
        let best_wrong_score = ranked
            .iter()
            .filter(|a| a.candidate_id != case.correct_candidate_id)
            .map(|a| a.ranking_score.unwrap())
            .fold(f64::NEG_INFINITY, f64::max);
        let margin = correct.ranking_score.unwrap() - best_wrong_score;
        margins.push(margin);
        coverages.push(evidence_coverage(correct));

        total_candidates += ranked.len();
        net_excluded += ranked
            .iter()
            .filter(|a| a.ranking_score.unwrap_or(0.0) <= 0.0)
            .count();

        println!(
            "{:<15} rank={rank} score={:.2} margin={:.2}{}",
            case.name,
            correct.ranking_score.unwrap(),
            margin,
            if is_known_tie {
                " (known isomer tie)"
            } else {
                ""
            }
        );

        // Cases outside the known-tie list must strictly beat every
        // decoy, exactly as required since the 3-case v0.1 corpus --
        // zero regression tolerance here. Known-tie cases must never do
        // *worse* than their decoy (margin >= 0), and the tie must be
        // exact (not some other, unexplained discrepancy).
        if is_known_tie {
            assert_eq!(
                margin, 0.0,
                "{}: expected an exact, documented tie with its decoy",
                case.name
            );
        } else {
            assert!(
                margin > 0.0,
                "{}: correct candidate must strictly beat every decoy",
                case.name
            );
        }
    }

    let top_1_accuracy = top_1_hits as f64 / n;
    let top_2_recall = top_2_hits as f64 / n;
    let mrr = reciprocal_ranks.iter().sum::<f64>() / n;
    let mean_margin = margins.iter().sum::<f64>() / n;
    let mean_coverage = coverages.iter().sum::<f64>() / n;
    let candidate_reduction = net_excluded as f64 / total_candidates as f64;

    println!(
        "top_1_accuracy={top_1_accuracy:.2} top_2_recall={top_2_recall:.2} mrr={mrr:.2} \
         mean_margin={mean_margin:.2} mean_evidence_coverage={mean_coverage:.2} \
         candidate_reduction={candidate_reduction:.2} (n={n})"
    );

    // Corpus-wide assertions. top_1_accuracy/mrr are intentionally NOT
    // required to be 1.0 across all 6 cases: 3 of them are known,
    // documented isomer ties (see KNOWN_ISOMER_TIES) where this
    // evaluator set genuinely cannot discriminate the correct candidate
    // from its decoy, and a tied rank is an artifact of stable-sort
    // insertion order, not a real #1 finish. top_2_recall staying at 1.0
    // is the honest version of the old "every known adduct ranks first"
    // guarantee for this corpus: every correct candidate is still never
    // outranked by its decoy, even in a tie.
    assert_eq!(
        top_2_recall, 1.0,
        "every known adduct must be at worst tied for first in its own case"
    );
    assert!(
        mean_margin > 0.0,
        "correct candidate must beat the best decoy on average"
    );
    assert!(mean_coverage > 0.0);
    // At least one decoy across the whole corpus should be net-excluded
    // (ranking_score <= 0) -- if nothing is ever excluded, the evidence
    // engine isn't discriminating at all.
    assert!(
        candidate_reduction > 0.0,
        "expected at least one net-excluded decoy across the corpus"
    );
}
