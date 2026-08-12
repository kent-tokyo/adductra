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

Reflects the actual tree, not the original plan — kept in sync as the
crate grows (`neutral_loss.rs` was folded into `fragment.rs` since both
reduce to the same rule-matching mechanics; `candidate_gen/exact_mass.rs`
was never built, deliberately, per `ROADMAP.md` Phase 8 — v0.1's
milestone only needed user-supplied candidates).

```text
src/
  lib.rs                  public prelude / re-exports, crate-level no-panic lints
  error.rs                AdductraError (public error type, no panics in library code)
  evaluator.rs            EvidenceEvaluator trait + tolerance_strength (shared
                           strength-banding heuristic, used by mass/isotope evaluators)
  candidate_generator.rs  CandidateGenerator trait + UserSuppliedGenerator
  chem_adapter.rs         thin adapter over `chematic` (SMILES/formula string parsing
                           only). The only module allowed to import `chematic` types
                           directly. Does NOT use `chematic::chem::exact_mass` — see below.
  mass_table.rs           Adductra-owned monoisotopic isotope mass constants,
                           Formula type, formula→mass, ppm_error
  rules.rs                FragmentRule / RuleTarget / RuleExpectation + built_in_rules()
                           (loads rules/dna_adduct_fragments.json via include_str!)
  ranking.rs              Ranker (deterministic weighted aggregation) + explain()
  model/
    mod.rs
    numeric.rs            FiniteF64, NonNegativeF64 (validated newtypes)
    observation.rs        Observation, ProductIon, IsotopeLabel, IonAdductType
    candidate.rs          AdductCandidate, NucleobaseOrigin
    evidence.rs           Evidence, EvidenceKind, EvidenceDirection, EvidenceStrength,
                           EvidenceDetail, MissingReason, EvidenceSet
    assessment.rs         CandidateAssessment, AdductReport
    provenance.rs         Provenance, EvidenceSource
  evidence/
    mod.rs
    mass.rs                Mass + PrecursorConsistency evaluator
    fragment.rs             DiagnosticFragment + NeutralLoss evaluator (rule-driven)
    isotope.rs               IsotopeLabel evaluator (Phase 5)
  bin/
    adductra.rs           CLI (`rank` / `explain`), thin wrapper over the library API

rules/
  dna_adduct_fragments.json  versioned fragment/neutral-loss rule data (§13)

examples/
  bench_ranking.rs        §24 perf harness (100/1k/10k candidates, std::time only)
  masstrust_handoff.rs    demonstrates exporting an AdductReport to masstrust's CSV
                           input format (not a src/ public API — see its module doc)

tests/
  properties.rs               property-based tests (proptest)
  eight_oxo_dg_benchmark.rs   reference case 1: 8-oxo-dG (guanine)
  afb1_n7_gua_benchmark.rs    reference case 2: AFB1-N7-Gua / AFB1-FapyGua (guanine)
  ethenoadenine_benchmark.rs  reference case 3: 1,N6-ethenoadenine (adenine — the
                              first non-guanine case, deliberately added zero new
                              rule data as a generalization test)
  benchmark_corpus.rs         §15 corpus metrics across all three reference cases
  cli.rs                      CLI smoke tests
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
- **Rule targeting is coarse by design, not by oversight.** `RuleTarget`
  only has `Any` / `NucleobaseOrigin` / `CandidateId` — no "modification
  type" dimension. `NucleobaseOrigin::Other(String)` is the current
  workaround for narrowing a rule to a specific modification (e.g.
  `Other("8-oxo-guanine")`, so 8-oxo-dG-specific CO-loss rules don't also
  fire on other guanine-derived adducts like AFB1-N7-Gua — a real bug
  this exact confusion caused once, see `docs/benchmark.md`). Treat this
  as a load-bearing convention, not a suggestion, when adding new
  modification-specific rules.
- **Public library API vs. examples: a deliberate boundary.**
  `masstrust_handoff.rs` lives in `examples/`, not `src/`, specifically
  so Adductra's crate API isn't coupled to a still-evolving sibling
  crate's CSV schema. The same reasoning applies to any future
  sibling-crate integration: prefer an example demonstrating the
  hand-off over a permanent public export until the other side's format
  has stabilized.
- **No panics in library code.** All fallible paths return
  `Result<_, AdductraError>`. `unwrap`/`expect`/`panic!` are forbidden
  outside of tests (`#![forbid(...)]` where practical).
