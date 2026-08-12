# Changelog

All notable changes to Adductra are documented here.

Format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/);
versioning follows [Semantic Versioning](https://semver.org/) once a
first version ships. Nothing has been published to crates.io yet — see
`ROADMAP.md`'s "v0.1.0 readiness review" for release status.

## [Unreleased]

### Added

- Core evidence model: `Observation`, `AdductCandidate`, `Evidence` /
  `EvidenceKind` / `EvidenceDirection` / `EvidenceStrength` /
  `EvidenceDetail` / `EvidenceSet`, `CandidateAssessment`, `AdductReport`,
  `Provenance` — no single score ever absorbs evidence; every evaluator
  returns structured, inspectable `Evidence`.
- Mass and precursor-consistency evidence (`MassEvidenceEvaluator`):
  exact-mass / ppm-error checks, configurable tolerance, multiply-charged
  precursor handling (`[M+2H]2+` and similar), and a hard charge-sign /
  ion-adduct polarity self-consistency check.
- Rule-driven diagnostic-fragment and neutral-loss evidence
  (`FragmentEvidenceEvaluator`), backed by versioned rule data
  (`rules/dna_adduct_fragments.json`) rather than hard-coded chemistry —
  distinguishes `Missing` (not measured) from `Contradicting` (measured
  but absent) per the absence-of-evidence-is-not-evidence-of-absence
  principle.
- Isotope-labeling evidence (`IsotopeEvidenceEvaluator`): ¹³C/¹⁵N/D/¹⁸O
  label support, with impossible label counts (more labeled atoms than
  the candidate's formula has) rejected explicitly.
- Transparent, deterministic weighted-evidence ranking (`Ranker`, with
  configurable per-`EvidenceKind` weights) and structured + text
  explanation (`explain`) — ranking score is never presented as a
  calibrated probability.
- `CandidateGenerator` trait with a `UserSuppliedGenerator` baseline.
- `adductra` CLI (`rank`, `explain` subcommands; JSON `Observation` /
  `AdductCandidate` input, not vendor spectrum formats).
- Three independently-researched, hand-verified benchmark reference
  cases spanning two nucleobase families: 8-oxo-2'-deoxyguanosine,
  aflatoxin B1–N7-guanine / AFB1-FapyGua, and 1,N6-ethenoadenine — plus
  corpus-level metrics (top-1 accuracy, top-k recall, MRR, candidate
  reduction, ranking margin, evidence coverage).
- `examples/masstrust_handoff.rs`, demonstrating export to `masstrust`'s
  CSV format for downstream confidence/abstention.
- `examples/bench_ranking.rs`, a ranking performance harness
  (100/1k/10k candidates).
- CI: build, test (including doctests), clippy (deny warnings), fmt, and
  `cargo audit` dependency scanning.

### Design decisions worth knowing about

- Mass computation is owned by Adductra (`mass_table.rs`), not delegated
  to `chematic`: `chematic::chem::exact_mass` was found to be unreliable
  for isotope-labeled atoms. `chematic` is used only for SMILES/formula
  string parsing.
- Fragment/neutral-loss rules are versioned data, targeted by
  `NucleobaseOrigin` or `CandidateId` — a rule originally scoped too
  broadly (matching guanine-derived adducts generically) was found and
  fixed during benchmark development; see `docs/benchmark.md`.
