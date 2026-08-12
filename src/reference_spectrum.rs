//! Versioned reference-spectrum data fed to
//! [`crate::evidence::SpectralLibraryEvidenceEvaluator`] — the same shape
//! as [`crate::rules`] (external domain knowledge owned by the evaluator
//! that consumes it, not by [`crate::model::AdductCandidate`] or
//! [`crate::model::Observation`]).

use serde::{Deserialize, Serialize};

use crate::error::AdductraError;
use crate::model::{EvidenceSource, NonNegativeF64};

/// One peak in a reference spectrum. Unlike
/// [`crate::model::ProductIon`], `intensity` is mandatory — cosine
/// similarity needs it; use `1.0` for a presence-only peak list.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReferencePeak {
    pub mz: NonNegativeF64,
    pub intensity: NonNegativeF64,
}

impl ReferencePeak {
    pub fn new(mz: f64, intensity: f64) -> Result<Self, AdductraError> {
        Ok(Self {
            mz: NonNegativeF64::new(mz, "reference_peak.mz")?,
            intensity: NonNegativeF64::new(intensity, "reference_peak.intensity")?,
        })
    }
}

/// A known/published/predicted spectrum for one specific candidate,
/// matched by [`ReferenceSpectrum::candidate_id`] against
/// [`crate::model::AdductCandidate::id`]. Multiple entries per candidate
/// (different collision energy, instrument, or source) are expected —
/// [`crate::evidence::SpectralLibraryEvidenceEvaluator`] evaluates each
/// independently, producing one [`crate::model::Evidence`] item per entry
/// so per-entry provenance stays visible.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReferenceSpectrum {
    pub candidate_id: String,
    pub peaks: Vec<ReferencePeak>,
    pub source: EvidenceSource,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub citation: Option<String>,
    pub version: String,
    /// Free text, e.g. `"35 eV HCD"`. Copied into the emitted evidence's
    /// `Provenance.parameters`, not into `Observation` — this is a
    /// caveat about one comparison, not a property of what was measured.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub collision_energy: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub instrument: Option<String>,
}

impl ReferenceSpectrum {
    pub fn new(
        candidate_id: impl Into<String>,
        peaks: Vec<ReferencePeak>,
        source: EvidenceSource,
        version: impl Into<String>,
    ) -> Result<Self, AdductraError> {
        let candidate_id = candidate_id.into();
        if peaks.is_empty() {
            return Err(AdductraError::EmptyReferenceSpectrum { candidate_id });
        }
        Ok(Self {
            candidate_id,
            peaks,
            source,
            citation: None,
            version: version.into(),
            collision_energy: None,
            instrument: None,
        })
    }

    pub fn with_citation(mut self, citation: impl Into<String>) -> Self {
        self.citation = Some(citation.into());
        self
    }

    pub fn with_collision_energy(mut self, collision_energy: impl Into<String>) -> Self {
        self.collision_energy = Some(collision_energy.into());
        self
    }

    pub fn with_instrument(mut self, instrument: impl Into<String>) -> Self {
        self.instrument = Some(instrument.into());
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_peaks_rejected() {
        let result = ReferenceSpectrum::new("c1", vec![], EvidenceSource::Literature, "1.0.0");
        assert!(matches!(
            result,
            Err(AdductraError::EmptyReferenceSpectrum { .. })
        ));
    }

    #[test]
    fn builders_set_optional_fields() {
        let peak = ReferencePeak::new(100.0, 50.0).unwrap();
        let spectrum = ReferenceSpectrum::new("c1", vec![peak], EvidenceSource::Predicted, "1.0.0")
            .unwrap()
            .with_citation("Some Paper 2024")
            .with_collision_energy("35 eV HCD")
            .with_instrument("Orbitrap");
        assert_eq!(spectrum.citation.as_deref(), Some("Some Paper 2024"));
        assert_eq!(spectrum.collision_energy.as_deref(), Some("35 eV HCD"));
        assert_eq!(spectrum.instrument.as_deref(), Some("Orbitrap"));
    }
}
