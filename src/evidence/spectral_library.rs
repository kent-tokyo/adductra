//! Spectrum-vs-reference-spectrum similarity evidence — a holistic
//! comparison, distinct from `FragmentEvidenceEvaluator`'s single-peak
//! present/absent checks. No new dependency: cosine similarity over two
//! sparse peak lists is a greedy peak match plus two dot products.
//!
//! ponytail: modified cosine (shifted-mass matching, used to find
//! *related* spectra) is not implemented. It needs the reference
//! spectrum's own precursor mass (no other use for that field here), a
//! second matching pass combining direct and shifted candidate pairs,
//! and answers a different question (relatedness discovery) than this
//! evaluator does (does *this* candidate's own known spectrum match).
//! Add if a benchmark case needs it.

use crate::error::AdductraError;
use crate::evaluator::{EvidenceEvaluator, tolerance_strength};
use crate::model::{
    AdductCandidate, Evidence, EvidenceDetail, EvidenceKind, EvidenceSource, EvidenceStrength,
    MissingReason, NonNegativeF64, Observation, Provenance, SpectralSimilarityMetric,
};
use crate::reference_spectrum::{ReferencePeak, ReferenceSpectrum};

pub struct SpectralLibraryEvidenceEvaluator {
    reference_spectra: Vec<ReferenceSpectrum>,
    mz_tolerance_da: f64,
    similarity_threshold: f64,
}

impl SpectralLibraryEvidenceEvaluator {
    /// `similarity_threshold` must be in `(0.5, 1.0)`: outside that
    /// range, `tolerance_strength`'s shared strength-banding (reused
    /// here so this evaluator's heuristic can't silently drift from
    /// every other tolerance-based evaluator) degenerates — at or below
    /// 0.5, `Strong`-contradicting becomes unreachable; at or above 0.9,
    /// a real-world cosine of 0.79 gets banded `Strong`-contradicting,
    /// too aggressive for MS/MS data.
    pub fn new(
        reference_spectra: Vec<ReferenceSpectrum>,
        mz_tolerance_da: f64,
        similarity_threshold: f64,
    ) -> Result<Self, AdductraError> {
        NonNegativeF64::new(mz_tolerance_da, "mz_tolerance_da")?;
        if !(0.5..1.0).contains(&similarity_threshold) {
            return Err(AdductraError::InvalidSimilarityThreshold(
                similarity_threshold,
            ));
        }
        Ok(Self {
            reference_spectra,
            mz_tolerance_da,
            similarity_threshold,
        })
    }

    fn provenance_for(&self, reference: &ReferenceSpectrum) -> Provenance {
        let mut provenance = Provenance {
            software_version: env!("CARGO_PKG_VERSION").to_string(),
            rule_version: Some(reference.version.clone()),
            source_citation: reference.citation.clone(),
            algorithm_version: Some("spectral_library_evidence_evaluator_v1".to_string()),
            ..Default::default()
        }
        .with_parameter("mz_tolerance_da", self.mz_tolerance_da)
        .with_parameter("similarity_threshold", self.similarity_threshold);
        if let Some(ce) = &reference.collision_energy {
            provenance = provenance.with_parameter("collision_energy", ce.clone());
        }
        if let Some(instrument) = &reference.instrument {
            provenance = provenance.with_parameter("instrument", instrument.clone());
        }
        provenance
    }

    fn base_provenance(&self) -> Provenance {
        Provenance::derived("spectral_library_evidence_evaluator_v1")
            .with_parameter("mz_tolerance_da", self.mz_tolerance_da)
            .with_parameter("similarity_threshold", self.similarity_threshold)
    }
}

struct SpectralComparison {
    metric: SpectralSimilarityMetric,
    cosine_similarity: Option<f64>,
    matched_peak_fraction: f64,
    matched_peak_count: usize,
}

/// Greedy 1:1 peak match within `tol_da`, then sqrt-intensity cosine over
/// the *whole* spectrum (not just matched peaks — see module doc for why
/// that distinction matters).
fn compare_spectra(
    reference: &[ReferencePeak],
    observed: &[crate::model::ProductIon],
    tol_da: f64,
) -> SpectralComparison {
    let mut candidates: Vec<(usize, usize, f64, f64)> = Vec::new();
    for (ri, rp) in reference.iter().enumerate() {
        for (oi, op) in observed.iter().enumerate() {
            let delta = (rp.mz.get() - op.mz.get()).abs();
            if delta <= tol_da {
                let obs_intensity = op.intensity.map(|i| i.get()).unwrap_or(0.0);
                candidates.push((ri, oi, delta, rp.intensity.get() * obs_intensity));
            }
        }
    }
    // Closest delta first; ties by higher intensity product (chemically
    // meaningful, and independent of input order); final tie-break on
    // indices for full determinism.
    candidates.sort_by(|a, b| {
        a.2.total_cmp(&b.2)
            .then_with(|| b.3.total_cmp(&a.3))
            .then_with(|| a.0.cmp(&b.0))
            .then_with(|| a.1.cmp(&b.1))
    });

    let mut ref_used = vec![false; reference.len()];
    let mut obs_used = vec![false; observed.len()];
    let mut matched_pairs = Vec::new();
    for (ri, oi, ..) in candidates {
        if !ref_used[ri] && !obs_used[oi] {
            ref_used[ri] = true;
            obs_used[oi] = true;
            matched_pairs.push((ri, oi));
        }
    }
    let matched_peak_count = matched_pairs.len();
    // reference is never empty: ReferenceSpectrum::new rejects that.
    let matched_peak_fraction = matched_peak_count as f64 / reference.len() as f64;

    let ref_sqrt: Vec<f64> = reference.iter().map(|p| p.intensity.get().sqrt()).collect();
    let obs_sqrt: Vec<f64> = observed
        .iter()
        .map(|p| p.intensity.map(|i| i.get()).unwrap_or(0.0).sqrt())
        .collect();
    let ref_norm = ref_sqrt.iter().map(|x| x * x).sum::<f64>().sqrt();
    let obs_norm = obs_sqrt.iter().map(|x| x * x).sum::<f64>().sqrt();

    let cosine_similarity = if ref_norm > 0.0 && obs_norm > 0.0 {
        let dot: f64 = matched_pairs
            .iter()
            .map(|&(ri, oi)| ref_sqrt[ri] * obs_sqrt[oi])
            .sum();
        Some((dot / (ref_norm * obs_norm)).clamp(0.0, 1.0))
    } else {
        None
    };

    let metric = if cosine_similarity.is_some() {
        SpectralSimilarityMetric::Cosine
    } else {
        SpectralSimilarityMetric::MatchedPeakFraction
    };

    SpectralComparison {
        metric,
        cosine_similarity,
        matched_peak_fraction,
        matched_peak_count,
    }
}

impl EvidenceEvaluator for SpectralLibraryEvidenceEvaluator {
    fn evaluate(
        &self,
        observation: &Observation,
        candidate: &AdductCandidate,
    ) -> Result<Vec<Evidence>, AdductraError> {
        let matches: Vec<&ReferenceSpectrum> = self
            .reference_spectra
            .iter()
            .filter(|r| r.candidate_id == candidate.id)
            .collect();

        let what_was_tested = format!("spectral library match for {}", candidate.id);

        if matches.is_empty() {
            let detail = EvidenceDetail::SpectralLibraryMatch {
                metric: SpectralSimilarityMetric::MatchedPeakFraction,
                cosine_similarity: None,
                matched_peak_fraction: NonNegativeF64::new(0.0, "matched_peak_fraction")?,
                matched_peak_count: 0,
                reference_peak_count: 0,
                mz_tolerance_da: NonNegativeF64::new(self.mz_tolerance_da, "mz_tolerance_da")?,
            };
            return Ok(vec![Evidence::not_applicable(
                EvidenceKind::SpectralLibraryMatch,
                what_was_tested,
                detail,
                EvidenceSource::Derived,
                "no reference spectrum available for this candidate",
                self.base_provenance(),
            )]);
        }

        let has_ms2 = !observation.product_ions.is_empty();
        let mut evidence = Vec::with_capacity(matches.len());

        for reference in matches {
            let provenance = self.provenance_for(reference);
            let reference_peak_count = reference.peaks.len() as u32;

            if !has_ms2 {
                let detail = EvidenceDetail::SpectralLibraryMatch {
                    metric: SpectralSimilarityMetric::MatchedPeakFraction,
                    cosine_similarity: None,
                    matched_peak_fraction: NonNegativeF64::new(0.0, "matched_peak_fraction")?,
                    matched_peak_count: 0,
                    reference_peak_count,
                    mz_tolerance_da: NonNegativeF64::new(self.mz_tolerance_da, "mz_tolerance_da")?,
                };
                evidence.push(Evidence::missing(
                    EvidenceKind::SpectralLibraryMatch,
                    what_was_tested.clone(),
                    detail,
                    MissingReason::NotMeasured,
                    reference.source,
                    "no MS2 product ions in this observation",
                    provenance,
                ));
                continue;
            }

            let comparison = compare_spectra(
                &reference.peaks,
                &observation.product_ions,
                self.mz_tolerance_da,
            );
            let score = comparison
                .cosine_similarity
                .unwrap_or(comparison.matched_peak_fraction);
            let within = score >= self.similarity_threshold;
            let mut strength =
                tolerance_strength(1.0 - score, 1.0 - self.similarity_threshold, within);
            // AGENTS.md sec 26: predicted evidence must never look as
            // authoritative as experimental. A bad match against an
            // unreliable (predicted) reference shouldn't strongly demote
            // a candidate; a genuine match is still meaningful either
            // way, so Supporting strength is left uncapped.
            if !within
                && strength == EvidenceStrength::Strong
                && reference.source == EvidenceSource::Predicted
            {
                strength = EvidenceStrength::Moderate;
            }

            let detail = EvidenceDetail::SpectralLibraryMatch {
                metric: comparison.metric,
                cosine_similarity: comparison
                    .cosine_similarity
                    .map(|c| NonNegativeF64::new(c, "cosine_similarity"))
                    .transpose()?,
                matched_peak_fraction: NonNegativeF64::new(
                    comparison.matched_peak_fraction,
                    "matched_peak_fraction",
                )?,
                matched_peak_count: comparison.matched_peak_count as u32,
                reference_peak_count,
                mz_tolerance_da: NonNegativeF64::new(self.mz_tolerance_da, "mz_tolerance_da")?,
            };

            let method = "compare_spectra: greedy 1:1 peak match + sqrt-intensity cosine";
            evidence.push(if within {
                Evidence::supporting(
                    EvidenceKind::SpectralLibraryMatch,
                    what_was_tested.clone(),
                    detail,
                    strength,
                    reference.source,
                    method,
                    provenance,
                )
            } else {
                Evidence::contradicting(
                    EvidenceKind::SpectralLibraryMatch,
                    what_was_tested.clone(),
                    detail,
                    strength,
                    reference.source,
                    method,
                    provenance,
                )
            });
        }

        Ok(evidence)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{EvidenceDirection, IonAdductType, ProductIon, Provenance as Prov};

    fn candidate() -> AdductCandidate {
        AdductCandidate::from_formula("c1", "test", "C10H13N5O5", Prov::derived("fixture")).unwrap()
    }

    fn observation_with_peaks(peaks: Vec<(f64, f64)>) -> Observation {
        let mut obs = Observation::new("obs1", 284.0989, 1, IonAdductType::ProtonAdd).unwrap();
        obs.product_ions = peaks
            .into_iter()
            .map(|(mz, i)| ProductIon::new(mz, Some(i)).unwrap())
            .collect();
        obs
    }

    fn reference_for(id: &str, peaks: Vec<(f64, f64)>) -> ReferenceSpectrum {
        let peaks = peaks
            .into_iter()
            .map(|(mz, i)| ReferencePeak::new(mz, i).unwrap())
            .collect();
        ReferenceSpectrum::new(id, peaks, EvidenceSource::Literature, "1.0.0").unwrap()
    }

    #[test]
    fn no_reference_spectrum_is_not_applicable() {
        let evaluator = SpectralLibraryEvidenceEvaluator::new(vec![], 0.01, 0.7).unwrap();
        let evidence = evaluator
            .evaluate(&observation_with_peaks(vec![(100.0, 1.0)]), &candidate())
            .unwrap();
        assert_eq!(evidence.len(), 1);
        assert_eq!(evidence[0].direction(), EvidenceDirection::NotApplicable);
    }

    #[test]
    fn no_ms2_is_missing() {
        let reference = reference_for("c1", vec![(168.0516, 100.0)]);
        let evaluator = SpectralLibraryEvidenceEvaluator::new(vec![reference], 0.01, 0.7).unwrap();
        let obs = Observation::new("obs1", 284.0989, 1, IonAdductType::ProtonAdd).unwrap();
        let evidence = evaluator.evaluate(&obs, &candidate()).unwrap();
        assert_eq!(evidence[0].direction(), EvidenceDirection::Missing);
    }

    #[test]
    fn identical_spectrum_supports_with_cosine_one() {
        let peaks = vec![(168.0516, 100.0), (140.0567, 40.0), (112.0618, 15.0)];
        let reference = reference_for("c1", peaks.clone());
        let evaluator = SpectralLibraryEvidenceEvaluator::new(vec![reference], 0.01, 0.7).unwrap();
        let evidence = evaluator
            .evaluate(&observation_with_peaks(peaks), &candidate())
            .unwrap();
        assert_eq!(evidence.len(), 1);
        assert_eq!(evidence[0].direction(), EvidenceDirection::Supporting);
        assert!(matches!(
            evidence[0].detail(),
            EvidenceDetail::SpectralLibraryMatch { .. }
        ));
        if let EvidenceDetail::SpectralLibraryMatch {
            cosine_similarity, ..
        } = evidence[0].detail()
        {
            let cosine = cosine_similarity.unwrap().get();
            assert!((cosine - 1.0).abs() < 1e-9, "got {cosine}");
        }
    }

    #[test]
    fn completely_different_spectrum_contradicts() {
        let reference = reference_for("c1", vec![(168.0516, 100.0), (140.0567, 40.0)]);
        let evaluator = SpectralLibraryEvidenceEvaluator::new(vec![reference], 0.01, 0.7).unwrap();
        let evidence = evaluator
            .evaluate(
                &observation_with_peaks(vec![(500.0, 1.0), (600.0, 1.0)]),
                &candidate(),
            )
            .unwrap();
        assert_eq!(evidence[0].direction(), EvidenceDirection::Contradicting);
    }

    #[test]
    fn predicted_source_caps_contradicting_strength_at_moderate() {
        let peaks = vec![(168.0516, 100.0), (140.0567, 40.0)];
        let mut reference = reference_for("c1", peaks);
        reference.source = EvidenceSource::Predicted;
        let evaluator = SpectralLibraryEvidenceEvaluator::new(vec![reference], 0.01, 0.7).unwrap();
        let evidence = evaluator
            .evaluate(
                &observation_with_peaks(vec![(500.0, 1.0), (600.0, 1.0)]),
                &candidate(),
            )
            .unwrap();
        assert_eq!(evidence[0].direction(), EvidenceDirection::Contradicting);
        assert_eq!(evidence[0].strength(), Some(EvidenceStrength::Moderate));
    }

    #[test]
    fn invalid_similarity_threshold_rejected() {
        assert!(SpectralLibraryEvidenceEvaluator::new(vec![], 0.01, 0.3).is_err());
        assert!(SpectralLibraryEvidenceEvaluator::new(vec![], 0.01, 1.0).is_err());
    }

    #[test]
    fn zero_intensity_falls_back_to_matched_peak_fraction() {
        let reference = reference_for("c1", vec![(168.0516, 1.0)]);
        let evaluator = SpectralLibraryEvidenceEvaluator::new(vec![reference], 0.01, 0.7).unwrap();
        let mut obs = observation_with_peaks(vec![]);
        obs.product_ions = vec![ProductIon::new(168.0516, None).unwrap()];
        let evidence = evaluator.evaluate(&obs, &candidate()).unwrap();
        assert!(matches!(
            evidence[0].detail(),
            EvidenceDetail::SpectralLibraryMatch { .. }
        ));
        if let EvidenceDetail::SpectralLibraryMatch {
            metric,
            cosine_similarity,
            ..
        } = evidence[0].detail()
        {
            assert_eq!(*metric, SpectralSimilarityMetric::MatchedPeakFraction);
            assert!(cosine_similarity.is_none());
        }
    }
}
