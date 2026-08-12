//! `AGENTS.md` §10/§11/§29: a transparent, rule-based ranking baseline —
//! no ML model — plus the structured explanation that is Adductra's
//! actual reason to exist. `ranking_score` is a relative, unbounded
//! number, never a probability (§10, §29): compare candidates within one
//! report, don't compare scores across reports or treat them as
//! calibrated confidence.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::model::{
    CandidateAssessment, Evidence, EvidenceDetail, EvidenceDirection, EvidenceKind,
    EvidenceStrength,
};

fn strength_weight(strength: EvidenceStrength) -> f64 {
    match strength {
        EvidenceStrength::Weak => 1.0,
        EvidenceStrength::Moderate => 2.0,
        EvidenceStrength::Strong => 3.0,
    }
}

/// Transparent weighted-evidence ranker. `kind_weights` lets a caller
/// emphasize/de-emphasize an evidence type (e.g. weight isotope evidence
/// higher when a label was deliberately used); unlisted kinds default to
/// 1.0.
#[derive(Debug, Clone, Default)]
pub struct Ranker {
    kind_weights: BTreeMap<EvidenceKind, f64>,
}

impl Ranker {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_kind_weight(mut self, kind: EvidenceKind, weight: f64) -> Self {
        self.kind_weights.insert(kind, weight);
        self
    }

    fn kind_weight(&self, kind: &EvidenceKind) -> f64 {
        self.kind_weights.get(kind).copied().unwrap_or(1.0)
    }

    /// Sum of `±(kind_weight * strength_weight)` over evidence, positive
    /// for `Supporting`, negative for `Contradicting`. `Missing` /
    /// `Unavailable` / `NotApplicable` contribute nothing — absence of
    /// evidence is not evidence of absence (§25).
    pub fn score(&self, evidence: &crate::model::EvidenceSet) -> f64 {
        evidence
            .iter()
            .map(|e| {
                let magnitude = self.kind_weight(e.kind())
                    * strength_weight(e.strength().unwrap_or(EvidenceStrength::Weak));
                match e.direction() {
                    EvidenceDirection::Supporting => magnitude,
                    EvidenceDirection::Contradicting => -magnitude,
                    _ => 0.0,
                }
            })
            .sum()
    }

    /// Scores every assessment and returns them ordered highest-first.
    /// Ties break on `candidate_id` (ascending) so ordering is fully
    /// deterministic (`AGENTS.md` §16: candidate ordering determinism).
    pub fn rank(&self, assessments: Vec<CandidateAssessment>) -> Vec<CandidateAssessment> {
        let mut scored: Vec<CandidateAssessment> = assessments
            .into_iter()
            .map(|mut a| {
                a.ranking_score = Some(self.score(&a.evidence));
                a
            })
            .collect();
        scored.sort_by(|a, b| {
            let sa = a.ranking_score.unwrap_or(f64::NEG_INFINITY);
            let sb = b.ranking_score.unwrap_or(f64::NEG_INFINITY);
            sb.total_cmp(&sa)
                .then_with(|| a.candidate_id.cmp(&b.candidate_id))
        });
        scored
    }
}

/// Whether one rendered [`ExplanationLine`] argues for, against, or is
/// merely informational about (e.g. `Missing`/`NotApplicable`) a candidate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExplanationPolarity {
    Supporting,
    Contradicting,
    Informational,
}

/// One rendered line of a candidate's [`Explanation`], derived from a
/// single piece of evidence.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExplanationLine {
    pub polarity: ExplanationPolarity,
    pub text: String,
}

/// Structured explanation for one candidate — the first-class
/// representation (§11); `to_text()` is just one rendering of it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Explanation {
    pub candidate_id: String,
    pub ranking_score: Option<f64>,
    pub lines: Vec<ExplanationLine>,
}

impl Explanation {
    pub fn to_text(&self) -> String {
        let mut out = match self.ranking_score {
            Some(score) => format!(
                "Candidate {} ranked with score {score:.2} because:\n\n",
                self.candidate_id
            ),
            None => format!("Candidate {} (unranked):\n\n", self.candidate_id),
        };
        for line in &self.lines {
            let prefix = match line.polarity {
                ExplanationPolarity::Supporting => '+',
                ExplanationPolarity::Contradicting => '-',
                ExplanationPolarity::Informational => '·',
            };
            out.push_str(&format!("{prefix} {}\n", line.text));
        }
        out
    }
}

fn render_detail(detail: &EvidenceDetail, matched: bool) -> String {
    match detail {
        EvidenceDetail::Mass {
            delta_ppm,
            tolerance_ppm,
            ..
        } => format!(
            "within {:.2} ppm (tolerance {:.1} ppm)",
            delta_ppm.get().abs(),
            tolerance_ppm.get()
        ),
        EvidenceDetail::PrecursorConsistency {
            delta_ppm,
            tolerance_ppm,
            ..
        } => format!(
            "Δ {:.2} ppm (tolerance {:.1} ppm)",
            delta_ppm.get(),
            tolerance_ppm.get()
        ),
        EvidenceDetail::DiagnosticFragment { expected_mz, .. } => {
            if matched {
                format!("fragment at m/z {:.4} observed", expected_mz.get())
            } else {
                format!("fragment at m/z {:.4} not observed", expected_mz.get())
            }
        }
        EvidenceDetail::NeutralLoss {
            expected_delta_da, ..
        } => {
            if matched {
                format!("neutral loss of {:.4} Da observed", expected_delta_da.get())
            } else {
                format!(
                    "expected neutral loss of {:.4} Da not observed",
                    expected_delta_da.get()
                )
            }
        }
        EvidenceDetail::IsotopeLabel {
            expected_shift_da,
            label_count,
            ..
        } => {
            if matched {
                format!(
                    "isotope shift matched {label_count} labelled atom(s) (Δ {:.4} Da)",
                    expected_shift_da.get()
                )
            } else if label_count == &0 {
                "no isotope label used".to_string()
            } else {
                format!("expected isotope shift for {label_count} labelled atom(s) not observed")
            }
        }
        EvidenceDetail::Generic { expected, observed } => match observed {
            Some(obs) => format!("expected {expected}, observed {obs}"),
            None => format!("expected {expected}"),
        },
    }
}

fn explanation_line(evidence: &Evidence) -> ExplanationLine {
    let matched = matches!(evidence.direction(), EvidenceDirection::Supporting);
    let detail_text = render_detail(evidence.detail(), matched);
    let text = format!("{} — {}", evidence.what_was_tested(), detail_text);
    let polarity = match evidence.direction() {
        EvidenceDirection::Supporting => ExplanationPolarity::Supporting,
        EvidenceDirection::Contradicting => ExplanationPolarity::Contradicting,
        EvidenceDirection::Missing => ExplanationPolarity::Informational,
        EvidenceDirection::Unavailable => ExplanationPolarity::Informational,
        EvidenceDirection::NotApplicable => ExplanationPolarity::Informational,
    };
    let text = match evidence.direction() {
        EvidenceDirection::Missing => format!(
            "{text} (missing: {:?})",
            evidence
                .missing_reason()
                .unwrap_or(crate::model::MissingReason::NotMeasured)
        ),
        EvidenceDirection::Unavailable => format!("{text} (unavailable)"),
        EvidenceDirection::NotApplicable => format!("{text} (not applicable)"),
        _ => text,
    };
    ExplanationLine { polarity, text }
}

/// Build the structured explanation for one already-ranked assessment.
pub fn explain(assessment: &CandidateAssessment) -> Explanation {
    Explanation {
        candidate_id: assessment.candidate_id.clone(),
        ranking_score: assessment.ranking_score,
        lines: assessment.evidence.iter().map(explanation_line).collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{EvidenceSet, EvidenceSource, Provenance};

    fn ev(direction_supports: bool, strength: EvidenceStrength) -> Evidence {
        let detail = EvidenceDetail::Generic {
            expected: "x".into(),
            observed: Some("x".into()),
        };
        if direction_supports {
            Evidence::supporting(
                EvidenceKind::Mass,
                "t",
                detail,
                strength,
                EvidenceSource::Derived,
                "m",
                Provenance::derived("test"),
            )
        } else {
            Evidence::contradicting(
                EvidenceKind::Mass,
                "t",
                detail,
                strength,
                EvidenceSource::Derived,
                "m",
                Provenance::derived("test"),
            )
        }
    }

    #[test]
    fn stronger_support_outranks_weaker() {
        let ranker = Ranker::new();
        let strong = CandidateAssessment::new(
            "a",
            EvidenceSet::new(vec![ev(true, EvidenceStrength::Strong)]),
        );
        let weak = CandidateAssessment::new(
            "b",
            EvidenceSet::new(vec![ev(true, EvidenceStrength::Weak)]),
        );
        let ranked = ranker.rank(vec![weak, strong]);
        assert_eq!(ranked[0].candidate_id, "a");
        assert_eq!(ranked[1].candidate_id, "b");
    }

    #[test]
    fn contradicting_evidence_lowers_score() {
        let ranker = Ranker::new();
        let score_support = ranker.score(&EvidenceSet::new(vec![ev(
            true,
            EvidenceStrength::Moderate,
        )]));
        let score_contradict = ranker.score(&EvidenceSet::new(vec![ev(
            false,
            EvidenceStrength::Moderate,
        )]));
        assert!(score_support > 0.0);
        assert!(score_contradict < 0.0);
        assert_eq!(score_support, -score_contradict);
    }

    #[test]
    fn tie_breaks_deterministically_on_candidate_id() {
        let ranker = Ranker::new();
        let a = CandidateAssessment::new("b-candidate", EvidenceSet::default());
        let b = CandidateAssessment::new("a-candidate", EvidenceSet::default());
        let ranked = ranker.rank(vec![a, b]);
        assert_eq!(ranked[0].candidate_id, "a-candidate");
        assert_eq!(ranked[1].candidate_id, "b-candidate");
    }

    #[test]
    fn empty_evidence_scores_zero() {
        let ranker = Ranker::new();
        assert_eq!(ranker.score(&EvidenceSet::default()), 0.0);
    }

    #[test]
    fn kind_weight_scales_that_kinds_contribution_only() {
        let mass_evidence = ev(true, EvidenceStrength::Strong); // EvidenceKind::Mass
        let default_score = Ranker::new().score(&EvidenceSet::new(vec![mass_evidence.clone()]));

        let doubled = Ranker::new()
            .with_kind_weight(EvidenceKind::Mass, 2.0)
            .score(&EvidenceSet::new(vec![mass_evidence.clone()]));
        assert_eq!(doubled, default_score * 2.0);

        // Weighting a *different* kind must not touch Mass evidence's score.
        let unaffected = Ranker::new()
            .with_kind_weight(EvidenceKind::DiagnosticFragment, 100.0)
            .score(&EvidenceSet::new(vec![mass_evidence]));
        assert_eq!(unaffected, default_score);
    }
}
