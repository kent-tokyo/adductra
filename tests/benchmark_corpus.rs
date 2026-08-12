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
//! computing straightforward formulas over 2 known-answer cases. Revisit
//! when there's an actual A vs. B ranking comparison to run.
//!
//! Fixture data intentionally duplicates (small, ~15 lines each) the
//! setup in `tests/eight_oxo_dg_benchmark.rs` and
//! `tests/afb1_n7_gua_benchmark.rs` rather than importing from them —
//! those files are exercising specific evidence-type behaviors and
//! already pass; this file adds a corpus-level view without risking a
//! refactor of already-verified tests.

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

#[test]
fn corpus_metrics_meet_v01_baseline() {
    let cases = [eight_oxo_dg_case(), afb1_n7_gua_case()];
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
        margins.push(correct.ranking_score.unwrap() - best_wrong_score);
        coverages.push(evidence_coverage(correct));

        total_candidates += ranked.len();
        net_excluded += ranked
            .iter()
            .filter(|a| a.ranking_score.unwrap_or(0.0) <= 0.0)
            .count();

        println!(
            "{:<15} rank={rank} score={:.2} margin={:.2}",
            case.name,
            correct.ranking_score.unwrap(),
            margins.last().unwrap()
        );
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

    // Baseline assertions for a 2-case v0.1 corpus: both known adducts
    // must rank first with a positive margin over every decoy, and get
    // fully-evaluable evidence (no evidence-type gaps for the *correct*
    // candidate specifically). Tightening these as the corpus grows is
    // expected; regressing them should fail CI.
    assert_eq!(
        top_1_accuracy, 1.0,
        "every known adduct must rank #1 in its own case"
    );
    assert_eq!(top_2_recall, 1.0);
    assert_eq!(mrr, 1.0);
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
