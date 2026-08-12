//! `AGENTS.md` §5–§6: evidence is never collapsed into a single `f64`.
//! Every [`Evidence`] value keeps what was tested, what was expected, what
//! was observed, the difference, the tolerance, and whether it supports,
//! contradicts, or could not be evaluated at all — and *why* it could not.

use serde::{Deserialize, Serialize};

use super::numeric::{FiniteF64, NonNegativeF64};
use super::provenance::{EvidenceSource, Provenance};
use crate::error::AdductraError;

/// Which category of evidence this is. `Custom` is an escape hatch for a
/// new rule-driven evidence type that doesn't yet warrant a first-class
/// variant (`AGENTS.md` §9: new evidence types must not require breaking
/// existing evaluators).
///
/// `#[non_exhaustive]` since 0.2.0: new evidence kinds are an expected,
/// purely additive part of this crate's growth (§9) — external exhaustive
/// matches need a wildcard arm so a future variant doesn't force a
/// breaking release for callers too.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum EvidenceKind {
    Mass,
    PrecursorConsistency,
    DiagnosticFragment,
    NeutralLoss,
    IsotopeLabel,
    NucleobaseNucleosideOrigin,
    StructuralPlausibility,
    /// Spectrum-vs-reference-spectrum similarity — see
    /// [`crate::evidence::SpectralLibraryEvidenceEvaluator`].
    SpectralLibraryMatch,
    Custom(String),
}

/// The relationship between what was expected and what was observed.
///
/// `Missing`, `Unavailable`, and `NotApplicable` are deliberately distinct
/// from `Contradicting`: the absence of evidence is not evidence of
/// absence (`AGENTS.md` §25).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EvidenceDirection {
    /// Observation is consistent with / favors this candidate.
    Supporting,
    /// Observation is inconsistent with this candidate.
    Contradicting,
    /// Expected to be observable but wasn't found — see [`MissingReason`]
    /// for *why*, which determines how strongly this should count against
    /// the candidate (if at all).
    Missing,
    /// This evidence type could not be evaluated for reasons unrelated to
    /// the candidate (e.g. the required observation wasn't collected).
    Unavailable,
    /// This evidence type does not apply to this candidate/observation
    /// pairing at all (e.g. isotope evidence when no label was used).
    NotApplicable,
}

/// Qualifies the magnitude of a `Supporting`/`Contradicting` direction.
/// Meaningless (and structurally absent) for the other three directions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum EvidenceStrength {
    Weak,
    Moderate,
    Strong,
}

/// Why an expected observation was not seen. `AGENTS.md` §25 requires
/// distinguishing these instead of collapsing them into one boolean.
///
/// `MeasuredButAbsent` is the sharpest case: the acquisition genuinely
/// covered the expected m/z/RT window and found nothing there. Evaluators
/// should generally treat that as `Contradicting` evidence, not `Missing`
/// — this type exists so an evaluator that *does* choose to report it as
/// `Missing` (e.g. because absence at low abundance is still ambiguous)
/// can say precisely why.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MissingReason {
    NotMeasured,
    BelowThreshold,
    OutsideAcquisitionRange,
    MeasuredButAbsent,
}

/// Which similarity metric a [`EvidenceDetail::SpectralLibraryMatch`]
/// used. `MatchedPeakFraction` is always computable; `Cosine` requires
/// usable intensity data on both sides of the comparison.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SpectralSimilarityMetric {
    Cosine,
    MatchedPeakFraction,
}

/// Typed, evidence-kind-specific payload: what was tested, expected,
/// observed, and within what tolerance. `Generic` is the fallback for
/// `EvidenceKind::Custom` and for kinds without a dedicated evaluator yet.
///
/// `#[non_exhaustive]` since 0.2.0, for the same reason as `EvidenceKind`.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum EvidenceDetail {
    Mass {
        theoretical_da: FiniteF64,
        observed_da: FiniteF64,
        delta_ppm: FiniteF64,
        tolerance_ppm: NonNegativeF64,
    },
    PrecursorConsistency {
        expected_mz: FiniteF64,
        observed_mz: FiniteF64,
        delta_ppm: FiniteF64,
        tolerance_ppm: NonNegativeF64,
        charge: i8,
        ion_adduct: String,
    },
    DiagnosticFragment {
        expected_mz: FiniteF64,
        tolerance_da: NonNegativeF64,
        matched_mz: Option<FiniteF64>,
    },
    NeutralLoss {
        expected_delta_da: FiniteF64,
        tolerance_da: NonNegativeF64,
        observed_delta_da: Option<FiniteF64>,
    },
    IsotopeLabel {
        expected_shift_da: FiniteF64,
        tolerance_da: NonNegativeF64,
        observed_shift_da: Option<FiniteF64>,
        label_count: u8,
    },
    SpectralLibraryMatch {
        metric: SpectralSimilarityMetric,
        /// `None` when neither spectrum had usable intensity data (falls
        /// back to `MatchedPeakFraction` as the reported `metric`).
        cosine_similarity: Option<NonNegativeF64>,
        matched_peak_fraction: NonNegativeF64,
        matched_peak_count: u32,
        reference_peak_count: u32,
        mz_tolerance_da: NonNegativeF64,
    },
    Generic {
        expected: String,
        observed: Option<String>,
    },
}

/// One piece of evidence for or against a candidate. Fields are private;
/// construct via [`Evidence::supporting`], [`Evidence::contradicting`],
/// [`Evidence::missing`], [`Evidence::unavailable`], or
/// [`Evidence::not_applicable`] so the direction/strength/missing_reason
/// invariant can never be violated — including when deserializing from
/// external JSON (`AGENTS.md` §16: round-trip must be meaningful).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "RawEvidence", into = "RawEvidence")]
pub struct Evidence {
    kind: EvidenceKind,
    what_was_tested: String,
    detail: EvidenceDetail,
    direction: EvidenceDirection,
    strength: Option<EvidenceStrength>,
    missing_reason: Option<MissingReason>,
    source: EvidenceSource,
    method: String,
    provenance: Provenance,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct RawEvidence {
    kind: EvidenceKind,
    what_was_tested: String,
    detail: EvidenceDetail,
    direction: EvidenceDirection,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    strength: Option<EvidenceStrength>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    missing_reason: Option<MissingReason>,
    source: EvidenceSource,
    method: String,
    provenance: Provenance,
}

impl From<Evidence> for RawEvidence {
    fn from(e: Evidence) -> Self {
        Self {
            kind: e.kind,
            what_was_tested: e.what_was_tested,
            detail: e.detail,
            direction: e.direction,
            strength: e.strength,
            missing_reason: e.missing_reason,
            source: e.source,
            method: e.method,
            provenance: e.provenance,
        }
    }
}

impl TryFrom<RawEvidence> for Evidence {
    type Error = AdductraError;

    fn try_from(raw: RawEvidence) -> Result<Self, Self::Error> {
        Evidence::build(
            raw.kind,
            raw.what_was_tested,
            raw.detail,
            raw.direction,
            raw.strength,
            raw.missing_reason,
            raw.source,
            raw.method,
            raw.provenance,
        )
    }
}

#[allow(clippy::too_many_arguments)]
impl Evidence {
    fn build(
        kind: EvidenceKind,
        what_was_tested: String,
        detail: EvidenceDetail,
        direction: EvidenceDirection,
        strength: Option<EvidenceStrength>,
        missing_reason: Option<MissingReason>,
        source: EvidenceSource,
        method: String,
        provenance: Provenance,
    ) -> Result<Self, AdductraError> {
        let strength_expected = matches!(
            direction,
            EvidenceDirection::Supporting | EvidenceDirection::Contradicting
        );
        if strength_expected != strength.is_some() {
            return Err(AdductraError::InvalidEvidenceStrength(direction));
        }
        let reason_expected = matches!(direction, EvidenceDirection::Missing);
        if reason_expected != missing_reason.is_some() {
            return Err(AdductraError::InvalidMissingReason);
        }
        Ok(Self {
            kind,
            what_was_tested,
            detail,
            direction,
            strength,
            missing_reason,
            source,
            method,
            provenance,
        })
    }

    /// Each convenience constructor below builds the struct directly
    /// rather than going through the fallible `build()` validation path:
    /// the direction/strength/missing_reason combination is fixed by the
    /// function's own shape (e.g. `supporting` only accepts a mandatory
    /// `EvidenceStrength`, so `Some(strength)` is guaranteed by the type
    /// signature, not by a runtime check). Only deserialization from
    /// untrusted external data goes through `build()`.
    pub fn supporting(
        kind: EvidenceKind,
        what_was_tested: impl Into<String>,
        detail: EvidenceDetail,
        strength: EvidenceStrength,
        source: EvidenceSource,
        method: impl Into<String>,
        provenance: Provenance,
    ) -> Self {
        Self {
            kind,
            what_was_tested: what_was_tested.into(),
            detail,
            direction: EvidenceDirection::Supporting,
            strength: Some(strength),
            missing_reason: None,
            source,
            method: method.into(),
            provenance,
        }
    }

    pub fn contradicting(
        kind: EvidenceKind,
        what_was_tested: impl Into<String>,
        detail: EvidenceDetail,
        strength: EvidenceStrength,
        source: EvidenceSource,
        method: impl Into<String>,
        provenance: Provenance,
    ) -> Self {
        Self {
            kind,
            what_was_tested: what_was_tested.into(),
            detail,
            direction: EvidenceDirection::Contradicting,
            strength: Some(strength),
            missing_reason: None,
            source,
            method: method.into(),
            provenance,
        }
    }

    pub fn missing(
        kind: EvidenceKind,
        what_was_tested: impl Into<String>,
        detail: EvidenceDetail,
        reason: MissingReason,
        source: EvidenceSource,
        method: impl Into<String>,
        provenance: Provenance,
    ) -> Self {
        Self {
            kind,
            what_was_tested: what_was_tested.into(),
            detail,
            direction: EvidenceDirection::Missing,
            strength: None,
            missing_reason: Some(reason),
            source,
            method: method.into(),
            provenance,
        }
    }

    pub fn unavailable(
        kind: EvidenceKind,
        what_was_tested: impl Into<String>,
        detail: EvidenceDetail,
        source: EvidenceSource,
        method: impl Into<String>,
        provenance: Provenance,
    ) -> Self {
        Self {
            kind,
            what_was_tested: what_was_tested.into(),
            detail,
            direction: EvidenceDirection::Unavailable,
            strength: None,
            missing_reason: None,
            source,
            method: method.into(),
            provenance,
        }
    }

    pub fn not_applicable(
        kind: EvidenceKind,
        what_was_tested: impl Into<String>,
        detail: EvidenceDetail,
        source: EvidenceSource,
        method: impl Into<String>,
        provenance: Provenance,
    ) -> Self {
        Self {
            kind,
            what_was_tested: what_was_tested.into(),
            detail,
            direction: EvidenceDirection::NotApplicable,
            strength: None,
            missing_reason: None,
            source,
            method: method.into(),
            provenance,
        }
    }

    pub fn kind(&self) -> &EvidenceKind {
        &self.kind
    }
    pub fn what_was_tested(&self) -> &str {
        &self.what_was_tested
    }
    pub fn detail(&self) -> &EvidenceDetail {
        &self.detail
    }
    pub fn direction(&self) -> EvidenceDirection {
        self.direction
    }
    pub fn strength(&self) -> Option<EvidenceStrength> {
        self.strength
    }
    pub fn missing_reason(&self) -> Option<MissingReason> {
        self.missing_reason
    }
    pub fn source(&self) -> EvidenceSource {
        self.source
    }
    pub fn method(&self) -> &str {
        &self.method
    }
    pub fn provenance(&self) -> &Provenance {
        &self.provenance
    }
}

/// An ordered collection of [`Evidence`] for one candidate. Duplicate or
/// empty is valid (a candidate with no applicable evidence just has an
/// empty set); ranking treats that as "no signal", not an error.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct EvidenceSet(Vec<Evidence>);

impl EvidenceSet {
    pub fn new(evidence: Vec<Evidence>) -> Self {
        Self(evidence)
    }

    pub fn iter(&self) -> impl Iterator<Item = &Evidence> {
        self.0.iter()
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn supporting(&self) -> impl Iterator<Item = &Evidence> {
        self.0
            .iter()
            .filter(|e| e.direction() == EvidenceDirection::Supporting)
    }

    pub fn contradicting(&self) -> impl Iterator<Item = &Evidence> {
        self.0
            .iter()
            .filter(|e| e.direction() == EvidenceDirection::Contradicting)
    }

    pub fn by_kind<'a>(&'a self, kind: &'a EvidenceKind) -> impl Iterator<Item = &'a Evidence> {
        self.0.iter().filter(move |e| e.kind() == kind)
    }

    pub fn push(&mut self, evidence: Evidence) {
        self.0.push(evidence);
    }
}

impl FromIterator<Evidence> for EvidenceSet {
    fn from_iter<I: IntoIterator<Item = Evidence>>(iter: I) -> Self {
        Self(iter.into_iter().collect())
    }
}

impl IntoIterator for EvidenceSet {
    type Item = Evidence;
    type IntoIter = std::vec::IntoIter<Evidence>;
    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn prov() -> Provenance {
        Provenance::derived("test")
    }

    #[test]
    fn smart_constructors_enforce_invariant() {
        let e = Evidence::supporting(
            EvidenceKind::Mass,
            "precursor exact mass",
            EvidenceDetail::Generic {
                expected: "168.0510".into(),
                observed: Some("168.0511".into()),
            },
            EvidenceStrength::Strong,
            EvidenceSource::Derived,
            "test",
            prov(),
        );
        assert_eq!(e.direction(), EvidenceDirection::Supporting);
        assert_eq!(e.strength(), Some(EvidenceStrength::Strong));
        assert_eq!(e.missing_reason(), None);
    }

    #[test]
    fn raw_deserialize_rejects_strength_on_missing_direction() {
        let bad = r#"{
            "kind": "Mass",
            "what_was_tested": "x",
            "detail": {"type": "Generic", "expected": "1", "observed": null},
            "direction": "Missing",
            "strength": "Strong",
            "missing_reason": "NotMeasured",
            "source": "Derived",
            "method": "test",
            "provenance": {"software_version": "0.1.0"}
        }"#;
        let result: Result<Evidence, _> = serde_json::from_str(bad);
        assert!(result.is_err());
    }

    #[test]
    fn round_trip_preserves_value() {
        let e = Evidence::missing(
            EvidenceKind::DiagnosticFragment,
            "guanine-derived fragment",
            EvidenceDetail::DiagnosticFragment {
                expected_mz: FiniteF64::new(152.0567, "expected_mz").unwrap(),
                tolerance_da: NonNegativeF64::new(0.01, "tolerance_da").unwrap(),
                matched_mz: None,
            },
            MissingReason::BelowThreshold,
            EvidenceSource::Experimental,
            "MS2 peak list scan",
            prov(),
        );
        let json = serde_json::to_string(&e).unwrap();
        let back: Evidence = serde_json::from_str(&json).unwrap();
        assert_eq!(e, back);
    }

    #[test]
    fn evidence_set_empty_is_valid() {
        let set = EvidenceSet::default();
        assert!(set.is_empty());
        assert_eq!(set.supporting().count(), 0);
    }

    #[test]
    fn by_kind_filters_to_only_matching_evidence() {
        let mass_ev = Evidence::supporting(
            EvidenceKind::Mass,
            "mass",
            EvidenceDetail::Generic {
                expected: "x".into(),
                observed: None,
            },
            EvidenceStrength::Strong,
            EvidenceSource::Derived,
            "m",
            prov(),
        );
        let fragment_ev = Evidence::contradicting(
            EvidenceKind::DiagnosticFragment,
            "fragment",
            EvidenceDetail::Generic {
                expected: "y".into(),
                observed: None,
            },
            EvidenceStrength::Weak,
            EvidenceSource::Derived,
            "f",
            prov(),
        );
        let set = EvidenceSet::new(vec![mass_ev, fragment_ev]);

        let mass_only: Vec<_> = set.by_kind(&EvidenceKind::Mass).collect();
        assert_eq!(mass_only.len(), 1);
        assert_eq!(*mass_only[0].kind(), EvidenceKind::Mass);

        assert_eq!(set.by_kind(&EvidenceKind::NeutralLoss).count(), 0);
    }
}
