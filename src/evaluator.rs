//! `AGENTS.md` §9: evidence evaluation is modular — one `EvidenceEvaluator`
//! implementation per evidence type, addable without touching existing
//! evaluators or the ranking logic.

use crate::error::AdductraError;
use crate::model::{AdductCandidate, Evidence, EvidenceStrength, Observation};

pub trait EvidenceEvaluator {
    fn evaluate(
        &self,
        observation: &Observation,
        candidate: &AdductCandidate,
    ) -> Result<Vec<Evidence>, AdductraError>;
}

/// Bands a match/mismatch by how far `abs_delta` is from `tolerance`.
/// Shared by every tolerance-based evaluator (mass, precursor, isotope)
/// so their strength heuristics can't silently drift apart. `within`
/// must be `abs_delta <= tolerance` (callers already computed this).
pub(crate) fn tolerance_strength(abs_delta: f64, tolerance: f64, within: bool) -> EvidenceStrength {
    if within {
        if abs_delta <= tolerance / 2.0 {
            EvidenceStrength::Strong
        } else {
            EvidenceStrength::Moderate
        }
    } else if abs_delta <= tolerance * 2.0 {
        EvidenceStrength::Weak
    } else {
        EvidenceStrength::Strong
    }
}
