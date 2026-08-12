//! Adductra's public error type.

use thiserror::Error;

/// Public error type for Adductra. Library code never panics on
/// recoverable input errors (`AGENTS.md` §23) — invalid input always
/// surfaces here instead.
#[derive(Debug, Error, Clone, PartialEq)]
pub enum AdductraError {
    #[error("non-finite value not allowed for {field}: {value}")]
    NonFinite { field: &'static str, value: f64 },

    #[error("negative value not allowed for {field}: {value}")]
    Negative { field: &'static str, value: f64 },

    #[error("invalid charge: {0} (charge must be a non-zero integer)")]
    InvalidCharge(i8),

    #[error("invalid SMILES {smiles:?}: {reason}")]
    InvalidSmiles { smiles: String, reason: String },

    #[error("invalid molecular formula {formula:?}: {reason}")]
    InvalidFormula { formula: String, reason: String },

    #[error("unknown element symbol: {0}")]
    UnknownElement(String),

    #[error("invalid rule data in {file}: {reason}")]
    InvalidRuleData { file: String, reason: String },

    #[error(
        "impossible isotope label: {requested} labeled {element} atoms requested but candidate formula only has {available}"
    )]
    ImpossibleIsotopeCount {
        element: String,
        requested: u8,
        available: u32,
    },

    #[error("adduct candidate must specify at least one of: smiles, formula")]
    CandidateMissingStructureAndFormula,

    #[error("evidence strength must be present when direction is {0:?} and absent otherwise")]
    InvalidEvidenceStrength(super::model::EvidenceDirection),

    #[error("missing_reason must be present when direction is Missing and absent otherwise")]
    InvalidMissingReason,
}
