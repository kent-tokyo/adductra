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

Pre-release, under active development. See `ROADMAP.md` for phase progress
and `docs/landscape.md` for the Phase 0 design survey.

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
