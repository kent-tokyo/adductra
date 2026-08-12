//! Adductra's core data model (§5): observations, candidates, evidence,
//! assessments, and provenance.

mod assessment;
mod candidate;
mod evidence;
mod numeric;
mod observation;
mod provenance;

pub use assessment::{AdductReport, CandidateAssessment};
pub use candidate::{AdductCandidate, NucleobaseOrigin};
pub use evidence::{
    Evidence, EvidenceDetail, EvidenceDirection, EvidenceKind, EvidenceSet, EvidenceStrength,
    MissingReason, SpectralSimilarityMetric,
};
pub use numeric::{FiniteF64, NonNegativeF64};
pub use observation::{IonAdductType, IsotopeLabel, Observation, ProductIon};
pub use provenance::{EvidenceSource, Provenance};
