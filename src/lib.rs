//! Adductra: an evidence-first Rust toolkit for identifying and explaining
//! DNA adduct candidates from mass-spectrometric and structural evidence.
//!
//! Adductra is a research tool. It does not diagnose cancer or establish
//! causal exposure.
//!
//! See `README.md` for the concept, `ARCHITECTURE.md` for module
//! boundaries, and `docs/landscape.md` for the Phase 0 design survey.
//!
//! # Example
//!
//! ```
//! use adductra::{
//!     AdductCandidate, CandidateAssessment, EvidenceEvaluator, EvidenceSet,
//!     IonAdductType, MassEvidenceEvaluator, NucleobaseOrigin, Observation,
//!     ProductIon, Provenance, Ranker, explain,
//! };
//!
//! // 8-oxo-2'-deoxyguanosine, observed as [M+H]+ at m/z 284.0989.
//! let observation = Observation::new("obs-1", 284.0989, 1, IonAdductType::ProtonAdd)
//!     .unwrap()
//!     .with_product_ions(vec![ProductIon::new(168.0516, Some(100.0)).unwrap()]);
//!
//! let candidate = AdductCandidate::from_formula(
//!     "8-oxo-dG",
//!     "8-oxo-2'-deoxyguanosine",
//!     "C10H13N5O5",
//!     Provenance::derived("doctest"),
//! )
//! .unwrap()
//! .with_nucleobase_origin(NucleobaseOrigin::Other("8-oxo-guanine".to_string()));
//!
//! let mass_evaluator = MassEvidenceEvaluator::new(10.0).unwrap();
//! let evidence = mass_evaluator.evaluate(&observation, &candidate).unwrap();
//! let assessment = CandidateAssessment::new(candidate.id.clone(), EvidenceSet::new(evidence));
//! let ranked = Ranker::new().rank(vec![assessment]);
//!
//! assert!(ranked[0].ranking_score.unwrap() > 0.0);
//! println!("{}", explain(&ranked[0]).to_text());
//! ```
//!
//! See `examples/` for the full evidence-evaluator suite (mass,
//! fragment, isotope) and the `adductra` CLI (`src/bin/adductra.rs`).

#![forbid(unsafe_code)]
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]

pub mod candidate_generator;
pub mod chem_adapter;
pub mod error;
pub mod evaluator;
pub mod evidence;
pub mod mass_table;
pub mod model;
pub mod ranking;
pub mod reference_spectrum;
pub mod rules;

pub use candidate_generator::{CandidateGenerator, UserSuppliedGenerator};
pub use error::AdductraError;
pub use evaluator::EvidenceEvaluator;
pub use evidence::{
    FragmentEvidenceEvaluator, IsotopeEvidenceEvaluator, MassEvidenceEvaluator,
    SpectralLibraryEvidenceEvaluator,
};
pub use model::*;
pub use ranking::{Explanation, ExplanationLine, ExplanationPolarity, Ranker, explain};
pub use reference_spectrum::{ReferencePeak, ReferenceSpectrum};
pub use rules::{FragmentRule, RuleExpectation, RuleTarget};
