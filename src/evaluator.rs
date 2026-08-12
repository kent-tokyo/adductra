//! `AGENTS.md` §9: evidence evaluation is modular — one `EvidenceEvaluator`
//! implementation per evidence type, addable without touching existing
//! evaluators or the ranking logic.

use crate::error::AdductraError;
use crate::model::{AdductCandidate, Evidence, Observation};

pub trait EvidenceEvaluator {
    fn evaluate(
        &self,
        observation: &Observation,
        candidate: &AdductCandidate,
    ) -> Result<Vec<Evidence>, AdductraError>;
}
