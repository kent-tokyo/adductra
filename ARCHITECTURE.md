# Architecture

## Pipeline

```text
observations
     ↓
candidate generation   (trait CandidateGenerator)
     ↓
evidence extraction    (trait EvidenceEvaluator, one impl per EvidenceKind)
     ↓
evidence aggregation   (CandidateAssessment)
     ↓
candidate ranking      (transparent weighted aggregation, v0.1)
     ↓
explanation            (structured, serializable; text is a rendering of it)
```

## Module layout

```text
src/
  lib.rs              public prelude / re-exports
  error.rs            AdductraError (public error type, no panics in library code)
  model/
    observation.rs    Observation
    candidate.rs       AdductCandidate
    evidence.rs        Evidence, EvidenceKind, EvidenceDirection, EvidenceStrength,
                        EvidenceSource, EvidenceSet
    assessment.rs      CandidateAssessment, AdductReport
    provenance.rs      Provenance
  chem_adapter.rs      thin adapter over `chematic` (SMILES → Molecule → element-count
                       formula only). The only module allowed to import `chematic` types
                       directly. Does NOT use `chematic::chem::exact_mass` — see below.
  mass_table.rs        Adductra-owned monoisotopic isotope mass constants + formula→mass
  evidence/
    mass.rs            exact-mass / precursor-consistency evaluator
    fragment.rs         diagnostic-fragment evaluator
    neutral_loss.rs      neutral-loss evaluator (data-driven, see rules/)
    isotope.rs           isotope-labeling evaluator (Phase 5)
  candidate_gen/
    user_supplied.rs    accepts caller-provided candidates
    exact_mass.rs        generates candidates from a formula/mass search space
  ranking.rs            deterministic weighted aggregation + explain()
```

## Design boundaries

- **`chematic` isolation.** Only `chem_adapter.rs` imports `chematic::*`
  directly. Every other module works with Adductra's own types
  (`Formula`, `MonoisotopicMass`, ...) so `chematic`'s API can evolve or be
  swapped without touching evidence/ranking code. See `docs/landscape.md`
  §3 for the dependency decision.
- **Mass computation is NOT delegated to `chematic`.** Verified empirically
  (2026-08-12): `chematic::chem::exact_mass` treats an isotope-labeled
  atom's mass number as its mass in Daltons directly (`Atom.isotope: Some(13)`
  contributes `13.0`, not `13.003355`). Measured deltas: ¹³C off by
  ~0.0034 Da (~200 ppm on a 300 Da ion — far outside any usable tolerance),
  ¹⁵N off by ~1000×. This makes it unusable for isotope-labeling evidence
  (§7 P1) and untrustworthy for exact mass in general. Adductra therefore
  uses `chematic` only for SMILES parsing and element-count formula
  (`calc_mol_formula` / `formula_with_isotopes`, which just count atoms and
  are unaffected by the mass bug), and owns formula→mass conversion itself
  in `mass_table.rs` against NIST monoisotopic masses. Revisit if a future
  `chematic` release fixes this upstream.
- **Ranking score ≠ confidence.** `CandidateAssessment` carries a
  `ranking_score: f64` with no probabilistic interpretation. There is no
  `confidence` field in v0.1 — that is `masstrust`'s job, downstream of
  `AdductReport`.
- **No single score absorbs evidence.** Every evaluator returns
  `Vec<Evidence>` (not a scalar); aggregation happens once, visibly, in
  `ranking.rs`, and remains inspectable after the fact via
  `CandidateAssessment::evidence`.
- **Extensibility via traits, not a god-function.** New candidate sources
  implement `CandidateGenerator`; new evidence types implement
  `EvidenceEvaluator`. Neither trait knows about the other's implementors.
- **Rule data over hard-coded chemistry.** Neutral-loss / diagnostic
  fragment rules live as versioned data (see `AGENTS.md` §13), not as
  compound-specific `match` arms, so adding a new literature rule doesn't
  require touching evaluator logic.
- **No panics in library code.** All fallible paths return
  `Result<_, AdductraError>`. `unwrap`/`expect`/`panic!` are forbidden
  outside of tests (`#![forbid(...)]` where practical).
