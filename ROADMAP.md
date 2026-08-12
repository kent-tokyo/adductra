# Adductra Roadmap

Tracks progress against the development order defined in `AGENTS.md` §27
(v0.1) and §28 (post-v0.1). Check items off as they land; this file is the
task queue for autonomous development sessions — update it whenever a
phase item completes.

## Phase 0 — Landscape / design

- [x] Competitor / literature / data-source survey (`docs/landscape.md`)
- [x] Sibling-crate ecosystem check (chematic / risksieve / veridict / masstrust)
- [x] Benchmark reference case selection (8-oxo-dG, AFB1-N7-Gua; colibactin deferred)
- [x] `ARCHITECTURE.md` written
- [x] Public API sketch reviewed against `docs/landscape.md` findings (chematic dependency confirmed + isotope-mass bug found and designed around)

## Phase 1 — Core model

- [x] `Observation`
- [x] `AdductCandidate`
- [x] `Evidence` / `EvidenceKind` / `EvidenceDirection` / `EvidenceStrength` / `EvidenceSource` / `EvidenceSet`
- [x] `CandidateAssessment`
- [x] `AdductReport`
- [x] `Provenance`
- [x] Public error type (no `unwrap`/`expect`/`panic!` in library code — enforced by `#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic)]`)
- [x] serde serialization for all public types

## Phase 2 — Mass evidence

- [x] Formula / monoisotopic mass — formula parsing via `chematic` adapter, mass computation owned by Adductra (`src/mass_table.rs`) after finding `chematic::chem::exact_mass` is unreliable for labeled atoms
- [x] ppm error calculation (symmetric, well-defined — `mass_table::ppm_error`, property-tested)
- [x] Configurable tolerance
- [x] Precursor consistency (charge, ionization, adduct ion type — includes a hard polarity/charge-sign self-consistency check and correct handling of `|charge| > 1` via homogeneous adduct-stacking, e.g. `[M+2H]2+`)
- [x] Impossible isotope label counts (label count vs. atoms actually available in the candidate's formula) rejected with `AdductraError::ImpossibleIsotopeCount` rather than silently producing a wrong theoretical mass

## Phase 3 — Fragment evidence

- [x] Diagnostic fragment matching
- [x] Neutral loss representation (base loss, sugar-related loss, nucleoside fragmentation)
- [x] Data-driven rule representation (`rules/dna_adduct_fragments.json`, not hard-coded per compound)

## Phase 4 — Candidate ranking

- [x] Deterministic transparent weighted-evidence baseline (`src/ranking.rs`, `f64::total_cmp` + candidate_id tiebreak)
- [x] Explicit contradiction handling
- [x] `explain()` — structured + human-readable, JSON round-trip tested

## Phase 5 — Isotope evidence

- [x] Label representation (13C, 15N, D, 18O) — `IsotopeLabel` in `Observation`, folded into Mass evidence's theoretical mass
- [x] Expected shift calculation (`IsotopeLabel::total_shift_da`, property-tested for linearity)
- [ ] Dedicated `IsotopeEvidence` evaluator producing its own `EvidenceKind::IsotopeLabel` evidence (currently only feeds Mass evidence; not yet its own pass/fail check)

## Phase 6 — Benchmark

- [x] 8-oxo-dG reference case fixture (`tests/eight_oxo_dg_benchmark.rs`, `docs/benchmark.md`)
- [ ] AFB1-N7-Gua / FapyGua reference case fixture
- [x] Decoys / mass-close alternatives / missing-evidence cases — [ ] contradictory-evidence case (present-but-wrong fragment peak) still needed
- [ ] Top-k metrics, MRR, candidate reduction, ranking margin, evidence coverage (multi-case harness; only a single hand-written fixture exists so far)
- [ ] Regression fixtures beyond the one integration test

## Phase 7 — Ecosystem integration

- [x] `chematic` adapter finalized against real published API (structure/formula parsing only; mass computation deliberately NOT delegated — see `ARCHITECTURE.md`)
- [ ] `masstrust` hand-off format (candidate ranking → confidence/abstain)
- [ ] Optional `veridict` benchmark-evaluation integration

## Phase 8 — CLI / docs / release

- [ ] `adductra rank` CLI (thin wrapper over library API)
- [ ] `adductra explain` CLI / JSON output
- [x] `README.md`, `ARCHITECTURE.md`, `docs/evidence-model.md`, `docs/scoring.md`, `docs/provenance.md`, `docs/benchmark.md`
- [ ] crates.io metadata
- [ ] v0.1.0 readiness review — **requires approval before publish** (`AGENTS.md` §30)

---

## Cross-cutting (not phase-scoped in `AGENTS.md` §27, tracked here so it isn't lost)

- [ ] §24 performance benchmark harness (100 / 1,000 / 10,000 candidate ranking) — not started; no perf work done yet, deliberately (§24: don't optimize before there's a measured need)
- [ ] CI (build/test/clippy/fmt on push) — currently these all run locally, on demand, not automatically; nothing enforces them on a change that skips asking Claude to run them

## Post-v0.1 (not started; design-only per `AGENTS.md` §28)

- [ ] Python bindings (after Rust core stabilizes)
- [ ] WASM target (core already avoided fs/threads/native-only deps by design)
- [ ] Chemical exposure evidence linking
- [ ] Fragment prediction
- [ ] Isotope-assisted untargeted adductomics
- [ ] LC retention evidence
- [ ] Multi-stage MS
- [ ] Mutational signature bridge
- [ ] Cancer-specific research workflows

---

## Approval-gated items (do not execute without explicit sign-off)

- Publishing this repository publicly
- crates.io / PyPI publish
- Breaking API decisions with multiple reasonable options
- Breaking changes to other repos (chematic / risksieve / veridict / masstrust)
- External service credentials
- Large scope changes
- Any feature or wording implying clinical use
