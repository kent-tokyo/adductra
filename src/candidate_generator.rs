//! `AGENTS.md` §8: candidate generation is modular; v0.1 ships only the
//! user-supplied generator (the milestone in §"最初のゴール" only requires
//! accepting known candidates + decoys, not searching for them).

use crate::error::AdductraError;
use crate::model::{AdductCandidate, Observation};

/// Produces candidate DNA adducts for an observation. Implement this for
/// each new candidate source (§8); v0.1 ships only [`UserSuppliedGenerator`].
pub trait CandidateGenerator {
    /// Generate candidates for `observation`.
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Provenance;

    #[test]
    fn returns_exactly_the_supplied_candidates_regardless_of_observation() {
        let candidate =
            AdductCandidate::from_formula("c1", "test", "C1H4", Provenance::derived("test"))
                .unwrap();
        let generator = UserSuppliedGenerator::new(vec![candidate.clone()]);

        let obs_a =
            Observation::new("a", 100.0, 1, crate::model::IonAdductType::ProtonAdd).unwrap();
        let obs_b =
            Observation::new("b", 999.0, 2, crate::model::IonAdductType::ProtonLoss).unwrap();

        assert_eq!(generator.generate(&obs_a).unwrap(), vec![candidate.clone()]);
        assert_eq!(generator.generate(&obs_b).unwrap(), vec![candidate]);
    }

    #[test]
    fn empty_candidate_list_is_valid() {
        let generator = UserSuppliedGenerator::new(vec![]);
        let obs = Observation::new("a", 100.0, 1, crate::model::IonAdductType::ProtonAdd).unwrap();
        assert!(generator.generate(&obs).unwrap().is_empty());
    }
}
