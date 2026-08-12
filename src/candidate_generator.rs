//! `AGENTS.md` §8: candidate generation is modular; v0.1 ships only the
//! user-supplied generator (the milestone in §"最初のゴール" only requires
//! accepting known candidates + decoys, not searching for them).

use crate::error::AdductraError;
use crate::model::{AdductCandidate, Observation};

pub trait CandidateGenerator {
    fn generate(&self, observation: &Observation) -> Result<Vec<AdductCandidate>, AdductraError>;
}

/// Returns exactly the candidates the caller supplied, regardless of
/// `observation`. The trivial baseline generator.
pub struct UserSuppliedGenerator {
    candidates: Vec<AdductCandidate>,
}

impl UserSuppliedGenerator {
    pub fn new(candidates: Vec<AdductCandidate>) -> Self {
        Self { candidates }
    }
}

impl CandidateGenerator for UserSuppliedGenerator {
    fn generate(&self, _observation: &Observation) -> Result<Vec<AdductCandidate>, AdductraError> {
        Ok(self.candidates.clone())
    }
}
