//! `AGENTS.md` §24: measure `Ranker::rank` over 100 / 1,000 / 10,000
//! candidates. `std::time::Instant` only — no benchmarking crate added
//! for a single before/after timing loop.
//!
//! Run with `cargo run --release --example bench_ranking`.

use std::time::Instant;

use adductra::{
    CandidateAssessment, Evidence, EvidenceDetail, EvidenceKind, EvidenceSet, EvidenceSource,
    EvidenceStrength, Provenance, Ranker,
};

fn synthetic_assessment(i: usize) -> CandidateAssessment {
    let mut evidence = Vec::with_capacity(5);
    for k in 0..5u8 {
        let strength = match k % 3 {
            0 => EvidenceStrength::Strong,
            1 => EvidenceStrength::Moderate,
            _ => EvidenceStrength::Weak,
        };
        let detail = EvidenceDetail::Generic {
            expected: "x".to_string(),
            observed: Some("y".to_string()),
        };
        let ev = if k % 2 == 0 {
            Evidence::supporting(
                EvidenceKind::Mass,
                "synthetic",
                detail,
                strength,
                EvidenceSource::Derived,
                "bench",
                Provenance::derived("bench"),
            )
        } else {
            Evidence::contradicting(
                EvidenceKind::DiagnosticFragment,
                "synthetic",
                detail,
                strength,
                EvidenceSource::Derived,
                "bench",
                Provenance::derived("bench"),
            )
        };
        evidence.push(ev);
    }
    CandidateAssessment::new(format!("candidate-{i:06}"), EvidenceSet::new(evidence))
}

fn bench_n(n: usize) {
    let assessments: Vec<_> = (0..n).map(synthetic_assessment).collect();
    let ranker = Ranker::new();
    let start = Instant::now();
    let ranked = ranker.rank(assessments);
    let elapsed = start.elapsed();
    println!(
        "n={n:>6}  elapsed={elapsed:>10.2?}  top_score={:?}",
        ranked[0].ranking_score
    );
}

fn main() {
    for n in [100usize, 1_000, 10_000] {
        bench_n(n);
    }
}
