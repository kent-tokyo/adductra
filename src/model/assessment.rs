//! `AGENTS.md` §5/§11: the output of evaluating one candidate, and the
//! top-level report tying a ranked set of assessments back to the
//! observation that produced them.

use serde::{Deserialize, Serialize};

use super::evidence::EvidenceSet;
use super::provenance::Provenance;

/// Evidence-backed assessment of a single candidate. `ranking_score` is
/// `None` until ranking has run (Phase 4); it is a transparent, relative
/// score, never a probability or calibrated confidence
/// (`AGENTS.md` §10, §29).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CandidateAssessment {
    pub candidate_id: String,
    pub evidence: EvidenceSet,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub ranking_score: Option<f64>,
}

impl CandidateAssessment {
    pub fn new(candidate_id: impl Into<String>, evidence: EvidenceSet) -> Self {
        Self {
            candidate_id: candidate_id.into(),
            evidence,
            ranking_score: None,
        }
    }
}

/// Top-level output: every evaluated candidate for one observation,
/// ranked. `AGENTS.md` §11: explanation is a structured, serializable
/// first-class representation, not just rendered text.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AdductReport {
    pub observation_id: String,
    pub assessments: Vec<CandidateAssessment>,
    pub provenance: Provenance,
}
