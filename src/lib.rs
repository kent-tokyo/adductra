//! Adductra: an evidence-first Rust toolkit for identifying and explaining
//! DNA adduct candidates from mass-spectrometric and structural evidence.
//!
//! Adductra is a research tool. It does not diagnose cancer or establish
//! causal exposure.
//!
//! See `README.md` for the concept, `ARCHITECTURE.md` for module
//! boundaries, and `docs/landscape.md` for the Phase 0 design survey.

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
pub mod rules;

pub use candidate_generator::{CandidateGenerator, UserSuppliedGenerator};
pub use error::AdductraError;
pub use evaluator::EvidenceEvaluator;
pub use evidence::{FragmentEvidenceEvaluator, IsotopeEvidenceEvaluator, MassEvidenceEvaluator};
pub use model::*;
pub use ranking::{Explanation, ExplanationLine, ExplanationPolarity, Ranker, explain};
pub use rules::{FragmentRule, RuleExpectation, RuleTarget};
