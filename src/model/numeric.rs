//! Validated numeric newtypes (`AGENTS.md` §17: never trust a raw `f64`).
//!
//! `FiniteF64` and `NonNegativeF64` reject NaN/±inf (and negatives, for the
//! latter) at construction, including at deserialization time, so a
//! serialization round-trip is correct by construction rather than by test
//! discipline.

use serde::{Deserialize, Serialize};

use crate::error::AdductraError;

/// Any finite `f64` (no NaN, no ±inf). Used for signed quantities such as
/// ppm error or mass deltas, where negative values are meaningful.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
pub struct FiniteF64(f64);

impl FiniteF64 {
    pub fn new(value: f64, field: &'static str) -> Result<Self, AdductraError> {
        if value.is_finite() {
            Ok(Self(value))
        } else {
            Err(AdductraError::NonFinite { field, value })
        }
    }

    pub fn get(self) -> f64 {
        self.0
    }
}

impl<'de> Deserialize<'de> for FiniteF64 {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = f64::deserialize(deserializer)?;
        FiniteF64::new(value, "deserialized value").map_err(serde::de::Error::custom)
    }
}

/// A finite `f64` that is also `>= 0.0`. Used for masses, tolerances, and
/// intensities, where §17 explicitly requires rejecting negative values.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
pub struct NonNegativeF64(f64);

impl NonNegativeF64 {
    pub fn new(value: f64, field: &'static str) -> Result<Self, AdductraError> {
        if !value.is_finite() {
            return Err(AdductraError::NonFinite { field, value });
        }
        if value < 0.0 {
            return Err(AdductraError::Negative { field, value });
        }
        Ok(Self(value))
    }

    pub fn get(self) -> f64 {
        self.0
    }
}

impl<'de> Deserialize<'de> for NonNegativeF64 {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = f64::deserialize(deserializer)?;
        NonNegativeF64::new(value, "deserialized value").map_err(serde::de::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finite_rejects_nan_and_inf() {
        assert!(FiniteF64::new(f64::NAN, "x").is_err());
        assert!(FiniteF64::new(f64::INFINITY, "x").is_err());
        assert!(FiniteF64::new(f64::NEG_INFINITY, "x").is_err());
        assert!(FiniteF64::new(-3.5, "x").is_ok());
    }

    #[test]
    fn non_negative_rejects_negative() {
        assert!(NonNegativeF64::new(-0.001, "x").is_err());
        assert!(NonNegativeF64::new(0.0, "x").is_ok());
    }

    #[test]
    fn round_trip_via_json() {
        let v = NonNegativeF64::new(12.34, "x").unwrap();
        let json = serde_json::to_string(&v).unwrap();
        let back: NonNegativeF64 = serde_json::from_str(&json).unwrap();
        assert_eq!(v, back);

        // NaN cannot even be produced to serialize (rejected at construction),
        // and a hand-crafted NaN payload must fail to deserialize.
        let bad: Result<NonNegativeF64, _> = serde_json::from_str("NaN");
        assert!(bad.is_err());
        let bad_neg: Result<NonNegativeF64, _> = serde_json::from_str("-1.0");
        assert!(bad_neg.is_err());
    }
}
