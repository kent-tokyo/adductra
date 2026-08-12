# Adductra

Adductra is an evidence-first Rust toolkit for identifying and explaining
DNA adduct candidates from mass-spectrometric and structural evidence.

Adductra is a research tool.
It does not diagnose cancer or establish causal exposure.

Given MS/MS observations (precursor m/z, product ions, molecular formula,
optional isotope labels, optional structure hints), Adductra evaluates
candidate DNA adducts against multiple independent evidence types — exact
mass, precursor consistency, diagnostic fragments, neutral losses, isotope
labeling — and produces a ranked, **explainable** assessment. It never
collapses that evidence into a single opaque score, and it never presents a
ranking score as a calibrated probability.

```text
                   Adductra

       experimental observations
                 │
                 ▼
        candidate generation
                 │
                 ▼
       ┌─────────────────┐
       │ evidence engine │
       ├─────────────────┤
       │ exact mass      │
       │ MS/MS           │
       │ neutral losses  │
       │ isotope labels  │
       │ spectral library│
       │ structure       │
       └────────┬────────┘
                │
                ▼
          candidate rank
                │
                ▼
           explanation
                │
          ┌─────┴─────┐
          ▼           ▼
      researcher   masstrust
                       │
                       ▼
                calibrated trust
```

## Status

[![Crates.io](https://img.shields.io/crates/v/adductra.svg)](https://crates.io/crates/adductra)

v0.2.2, published to crates.io. Still under active development — see
`docs/landscape.md` for the Phase 0 design survey, `docs/benchmark.md`
for what's been validated so far, and `CHANGELOG.md` for release notes.

```toml
[dependencies]
adductra = "0.2"
```

## CLI

```bash
adductra rank    --observation obs.json --candidates candidates.json
adductra explain --observation obs.json --candidates candidates.json --candidate-id 8-oxo-dG [--json]

# optionally add spectral-library-match evidence against known reference spectra
adductra rank --observation obs.json --candidates candidates.json \
    --reference-spectra spectra.json \
    [--spectral-mz-tolerance-da 0.01] [--spectral-similarity-threshold 0.7]
```

`--observation` is a single JSON `Observation`, `--candidates` a JSON
array of `AdductCandidate`, `--reference-spectra` an optional JSON array
of `ReferenceSpectrum` — Adductra's own serde types, not a vendor
spectrum format (`.mgf`/raw-format parsing is deliberately out of scope;
see `src/bin/adductra.rs`'s module doc for the rationale).

## Ecosystem

Adductra composes rather than duplicates:

- [`chematic`](https://crates.io/crates/chematic) — molecular representation,
  SMILES parsing, exact mass / formula. Adductra depends on it directly.
- `masstrust` — downstream consumer for confidence / abstention on top of
  Adductra's `CandidateAssessment`. Not a dependency of Adductra.
- `risksieve` — general calibration / selective-prediction theory that
  `masstrust` builds on. Not a dependency of Adductra.
- `veridict` — statistical benchmark evaluation, used by Adductra's own
  benchmark harness, not embedded in the library.

## License

MIT OR Apache-2.0
