//! `AGENTS.md` §4/§10/§27 Phase 7: demonstrates handing a ranked
//! `AdductReport` off to `masstrust` for confidence/abstention, keeping
//! Adductra's own `ranking_score` uninterpreted as a probability the
//! whole way through.
//!
//! Verified against `masstrust`'s real source (`crates/masstrust-core/src/{types,io}.rs`
//! on GitHub, 2026-08-12), not guessed: its CSV reader requires exactly
//! `query_id,candidate_id,rank,score` and treats every other column
//! (`probability`, `smiles`, `inchikey`, `formula`, `is_correct`, ...) as
//! optional — several of its scoring methods (`ScoreGap`, `ScoreRatio`,
//! `TopKGap`, `CandidateCount`) work from `score` alone. So this hand-off
//! never fabricates a `probability` column; Adductra genuinely doesn't
//! have one to give (§29).
//!
//! ponytail: lives here as an example, not a `src/` public API function.
//! Committing a permanent library export to `masstrust`'s CSV schema
//! would couple Adductra's API surface to a still-evolving sibling
//! crate's format — a bigger, less reversible decision than a demo
//! needs to make. Promote to a real API (behind a feature flag, most
//! likely) once masstrust's schema has stabilized and there's a second
//! real consumer to design against.
//!
//! No `csv` crate dependency added for one demo — hand-written escaping
//! per RFC 4180 (quote a field if it contains a comma, quote, or
//! newline; double any internal quotes) is a few lines and avoids a new
//! dependency for output this simple.

use adductra::{
    AdductCandidate, AdductReport, CandidateAssessment, EvidenceEvaluator, EvidenceSet,
    FragmentEvidenceEvaluator, IonAdductType, IsotopeEvidenceEvaluator, MassEvidenceEvaluator,
    NucleobaseOrigin, Observation, ProductIon, Provenance, Ranker,
};

fn csv_field(value: &str) -> String {
    if value.contains(',') || value.contains('"') || value.contains('\n') {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.to_string()
    }
}

/// One masstrust `Candidate` CSV row for one ranked assessment.
/// `rank` is 1-indexed position in `report.assessments` (already sorted
/// by `Ranker::rank`); `probability` is deliberately never populated.
fn masstrust_csv_row(
    query_id: &str,
    rank: usize,
    assessment: &CandidateAssessment,
    candidate: &AdductCandidate,
) -> String {
    let score = assessment.ranking_score.unwrap_or(f64::NAN);
    let smiles = candidate.smiles.as_deref().unwrap_or("");
    format!(
        "{},{},{},{},,{},{}",
        csv_field(query_id),
        csv_field(&assessment.candidate_id),
        rank,
        score,
        csv_field(smiles), // masstrust column order: ...,probability,smiles,formula
        csv_field(&candidate.formula),
    )
}

fn to_masstrust_csv(report: &AdductReport, candidates: &[AdductCandidate]) -> String {
    let mut out = String::from("query_id,candidate_id,rank,score,probability,smiles,formula\n");
    for (i, assessment) in report.assessments.iter().enumerate() {
        let candidate = candidates
            .iter()
            .find(|c| c.id == assessment.candidate_id)
            .expect("report assessments must reference known candidates");
        out.push_str(&masstrust_csv_row(
            &report.observation_id,
            i + 1,
            assessment,
            candidate,
        ));
        out.push('\n');
    }
    out
}

fn main() {
    // The 8-oxo-dG reference case (tests/eight_oxo_dg_benchmark.rs),
    // reused here rather than imported — same rationale as
    // tests/benchmark_corpus.rs: this is demo code, not a shared
    // fixture, and duplicating ~20 lines avoids adding cross-file
    // coupling for it.
    let observation = Observation::new("obs-8oxodg-1", 284.0989, 1, IonAdductType::ProtonAdd)
        .unwrap()
        .with_product_ions(vec![
            ProductIon::new(168.0516, Some(100.0)).unwrap(),
            ProductIon::new(140.0567, Some(40.0)).unwrap(),
            ProductIon::new(112.0618, Some(15.0)).unwrap(),
        ]);
    let candidates = vec![
        // from_formula, not from_smiles: this demo doesn't need a
        // structure, and no SMILES for 8-oxo-dG has been verified
        // elsewhere in this codebase (see docs/landscape.md) — guessing
        // one here would risk a wrong structure shipping in example
        // code for a tool built around scientific correctness.
        AdductCandidate::from_formula(
            "8-oxo-dG",
            "8-oxo-2'-deoxyguanosine",
            "C10H13N5O5",
            Provenance::derived("masstrust-handoff-demo"),
        )
        .unwrap()
        .with_nucleobase_origin(NucleobaseOrigin::Other("8-oxo-guanine".to_string())),
        AdductCandidate::from_formula(
            "adenine-isomer",
            "isomeric adenine-derived decoy",
            "C10H13N5O5",
            Provenance::derived("masstrust-handoff-demo"),
        )
        .unwrap()
        .with_nucleobase_origin(NucleobaseOrigin::Adenine),
    ];

    let mass_evaluator = MassEvidenceEvaluator::new(10.0).unwrap();
    let fragment_evaluator = FragmentEvidenceEvaluator::with_built_in_rules().unwrap();
    let isotope_evaluator = IsotopeEvidenceEvaluator::new(0.005).unwrap();

    let assessments: Vec<CandidateAssessment> = candidates
        .iter()
        .map(|candidate| {
            let mut evidence = mass_evaluator.evaluate(&observation, candidate).unwrap();
            evidence.extend(
                fragment_evaluator
                    .evaluate(&observation, candidate)
                    .unwrap(),
            );
            evidence.extend(isotope_evaluator.evaluate(&observation, candidate).unwrap());
            CandidateAssessment::new(candidate.id.clone(), EvidenceSet::new(evidence))
        })
        .collect();

    let ranked = Ranker::new().rank(assessments);
    let report = AdductReport {
        observation_id: observation.id.clone(),
        assessments: ranked,
        provenance: Provenance::derived("masstrust-handoff-demo"),
    };

    print!("{}", to_masstrust_csv(&report, &candidates));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn csv_field_escapes_commas_quotes_and_newlines() {
        assert_eq!(csv_field("plain"), "plain");
        assert_eq!(csv_field("a,b"), "\"a,b\"");
        assert_eq!(csv_field("say \"hi\""), "\"say \"\"hi\"\"\"");
        assert_eq!(csv_field("line1\nline2"), "\"line1\nline2\"");
    }

    #[test]
    fn masstrust_csv_never_populates_probability() {
        let candidate =
            AdductCandidate::from_formula("c1", "test", "C1H1", Provenance::derived("test"))
                .unwrap();
        let assessment = {
            let mut a = CandidateAssessment::new("c1", EvidenceSet::default());
            a.ranking_score = Some(3.0);
            a
        };
        let row = masstrust_csv_row("q1", 1, &assessment, &candidate);
        // header: query_id,candidate_id,rank,score,probability,smiles,formula
        let fields: Vec<&str> = row.split(',').collect();
        assert_eq!(fields[4], "", "probability field must stay empty: {row}");
    }
}
