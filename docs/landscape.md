# Landscape Survey (Phase 0)

Research conducted 2026-08-12, before any implementation, per `AGENTS.md` §1.
Scope: DNA adductomics tooling, DNA-adduct reference databases, adjacent Rust
crates (including this author's own sibling-crate ecosystem), and selection
of a first benchmark reference case.

## 1. Existing DNA adductomics / adduct identification tools and pipelines

DNA adductomics is an active MS/MS sub-field, but tooling is thin — mostly
single-lab scripts rather than maintained open-source infrastructure.

- **nLossFinder** — open-source MATLAB GUI for nontargeted DNA/RNA adduct
  detection via characteristic neutral-loss tracking (deoxyribose/ribose
  loss) in HR-MS/MS; processes a sample in seconds.
  [PMC8067598](https://www.ncbi.nlm.nih.gov/pmc/articles/PMC8067598/),
  [PubMed 33916914](https://pubmed.ncbi.nlm.nih.gov/33916914/)
- **Wide-SIM/MS2 / DIA-based DNA·RNA adductomics workflows** — methodological
  (not distributed-software) pipelines on Orbitrap/Q-TOF.
  [PMC10523582](https://www.ncbi.nlm.nih.gov/pmc/articles/PMC10523582/),
  [PMC6822301](https://www.ncbi.nlm.nih.gov/pmc/articles/PMC6822301/)
- **Chemical-dynamics fragmentation prediction for O6-methylguanine** —
  tautomer enumeration + mobile-proton model validated against experimental
  HR spectra; no public software artifact found.
  [Springer](https://link.springer.com/article/10.1007/s13361-019-02348-7),
  [PubMed 31696434](https://pubmed.ncbi.nlm.nih.gov/31696434/)
- **MutAIverse** (2025 bioRxiv preprint) — closest existing analog to
  Adductra's ambition: models genotoxin bioactivation, builds a DNA adduct
  library, does spectral matching, and traces adducts back to parent
  genotoxins; validated on tobacco-associated head-and-neck cancer biopsies.
  Preprint-stage academic platform, not an installable typed library.
  [bioRxiv 2025.08.26.672508](https://www.biorxiv.org/content/10.1101/2025.08.26.672508v1.full)
- **General metabolomics adductomics tooling** (XCMS+CAMERA, OpenMS,
  MZmine/MS-DIAL, GNPS) is mature but targets *ionization adducts*
  ([M+Na]⁺ etc.), not covalent DNA-damage adducts — a naming false-friend
  worth flagging explicitly. No DNA-adduct-specific plugin found.
- **CFM-ID / MetFrag / SIRIUS** are general small-molecule fragmentation
  scorers, not DNA-adduct-adapted.
  [CFM-ID 4.0, NAR](https://academic.oup.com/nar/article/50/W1/W165/6591530)
- **Pytheas** — RNA-modification analog (not DNA): open-source Python tool
  for tandem-MS RNA modification analysis with isotope-label handling and
  decoy FDR. No comparable DNA equivalent found.
  [Nature Comms](https://www.nature.com/articles/s41467-022-30057-5)

## 2. DNA adduct / DNA damage reference databases

- **"A Comprehensive Database for DNA Adductomics"** (Frontiers in
  Chemistry, 2022) — 279 manually curated adducts (16 genotoxicant
  classes / 9 sources), 582 entries including combinatorial candidates.
  Reports monoisotopic mass, formula, SMILES, InChI, InChIKey, IUPAC name,
  plus a preliminary spectral library (15 standards × 3 collision
  energies) with experimental + in-silico fragments. Publicly downloadable:
  https://gitlab.com/nexs-metabolomics/projects/dna_adductomics_database.
  [Frontiers](https://www.frontiersin.org/journals/chemistry/articles/10.3389/fchem.2022.908572/full),
  [PMC9184683](https://www.ncbi.nlm.nih.gov/pmc/articles/PMC9184683/)
- **DNA Adduct Portal** (U. Minnesota + FIU/USF) — searchable HR MS2/MS3
  spectral library of adduct standards on Orbitrap and Q-TOF at multiple
  collision energies.
  [Portal](https://sites.google.com/umn.edu/dnaadductportal/home)
- **"Mass Spectral Library for DNA Adductomics"** (Chem. Res. Toxicol.,
  2023), companion publication.
  [ACS](https://pubs.acs.org/doi/abs/10.1021/acs.chemrestox.3c00302)
- No RepairToire-equivalent MS-evidence-centric database was found
  (RepairToire is pathway-, not spectrum-, centric).

**Conclusion**: the Frontiers/GitLab DB is the best seed for machine-readable
ground truth (SMILES/InChI/formula/mass + some real fragments); the DNA
Adduct Portal is the best source of real multi-CE MS2/MS3 spectra. Licensing
must be checked before any bulk ingestion into Adductra's own rule/data
files (§13).

## 3. Sibling-crate ecosystem check (chematic / risksieve / veridict / masstrust)

All four names in `AGENTS.md` §4 were checked against crates.io and GitHub
by a research pass (crates.io's own API was unreachable directly from
this environment — see caveat below). **Three are published, one is
GitHub-only — all four are authored by the same account, `kent-tokyo`.**
This is this project's own existing crate ecosystem, not third-party
prior art to avoid duplicating.

Directly verified from this session (not just the research pass):
`chematic` v0.14.0 is real, installable via `cargo add chematic`, and its
`chem`/`smiles` module APIs described below were read from the actual
downloaded source in `~/.cargo/registry/src/`. `masstrust`'s actual CSV
I/O schema (`crates/masstrust-core/src/{types,io}.rs`) was also directly
read from its GitHub source (2026-08-12, via `gh api`) while building
`examples/masstrust_handoff.rs` — see §Phase 7 and `docs/benchmark.md`.
The version numbers, download counts, and module lists for `risksieve`
and `veridict` below still come from the single research pass and were
**not** independently re-verified against the registry — treat those two
specifically as directionally correct (existence, ownership, rough
purpose) rather than exact.

| Crate | crates.io | GitHub | Status |
|---|---|---|---|
| `chematic` | v0.14.0, first published 2026-05-27, updated 2026-08-11 | `kent-tokyo/chematic` — targeting RDKit feature parity | Active, substantial |
| `risksieve` | v0.2.1 (v0.1.0 released 2026-07-27) | `kent-tokyo/risksieve` | Active |
| `veridict` | v0.19.0, first published 2026-07-04 | `kent-tokyo/veridict` | Active |
| `masstrust` | not published | `kent-tokyo/masstrust`, Apache-2.0/MIT | Real, source-only, CSV I/O schema directly verified |

Public API surface (from crate docs / README):

- **`chematic`** — modules `smiles`, `smarts`, `depict`, `mol`,
  `perception`, `fp` (ECFP/FCFP/MACCS/AtomPair), `chem` (MW, LogP, TPSA,
  exact/monoisotopic mass, isotope distributions), `rxn`, `inchi`, `threed`,
  `core`, `iupac`. Pure Rust, no C/C++ deps, WASM-capable. This already
  covers the molecular-representation / exact-mass substrate `AGENTS.md`
  §4 asks Adductra to reuse rather than reimplement.
- **`risksieve`** — modules `certificate` (`RiskCertificate`), `guarantee`,
  `loss` (`BoundedLoss`), `selective` (e-value abstention), `shift`,
  `anytime`, `crc`, `nonmonotone`; validated newtypes
  `OpenUnitInterval`/`ClosedUnitInterval`/`ClosedInterval`/`NonNegative`.
  Domain-agnostic, theorem-backed calibration/selective-prediction library.
- **`masstrust`** (unpublished) — "a Rust toolkit for selective prediction
  in MS/MS molecular annotation": takes candidate rankings from an external
  annotation tool and decides trust-vs-abstain at a target error rate. CLI
  verbs: `curve`, `calibrate`, `apply`, `compare`, `evaluate`, `drift`,
  `validate-split`, `certify-batch`. This is functionally the exact
  "confidence/calibration hand-off" sibling `AGENTS.md` §4 describes —
  MS/MS-flavored but not DNA-adduct-specific. Its real input schema
  (`Candidate` struct, `crates/masstrust-core/src/types.rs`): CSV columns
  `query_id`, `candidate_id`, `rank`, `score` are required; `probability`,
  `smiles`, `inchikey`, `formula`, `target_inchikey`, `is_correct` are all
  `Option` — several scoring methods (`ScoreGap`, `ScoreRatio`, `TopKGap`,
  `CandidateCount`) work from `score` alone, so a hand-off from Adductra
  never needs to fabricate a probability. See
  `examples/masstrust_handoff.rs`.
- **`veridict`** — modules `verdict`, `metrics`, `sprt`, `stats`, `report`,
  `matrix`, `plan`, `power`, `input`, `time_sensitive`, `verify_run`;
  win-rate/Elo/bootstrap-CI/SPRT statistical regression-gate library — fits
  §15 benchmark evaluation.

**Design decision for v0.1** (revisit if the real API diverges once
`chematic` is added as a dependency and inspected via `cargo doc`):

- Depend on **`chematic`** for molecular representation / formula / exact
  mass, behind a thin internal adapter module (`src/chem_adapter.rs`) so
  Adductra's own types never leak `chematic` types into evidence structs
  directly. This satisfies §4's "don't hard-couple before checking the
  real API" instruction while still avoiding a second mass-calculation
  engine.
- Do **not** depend on `risksieve` or `masstrust` in v0.1 core — ranking
  score vs. confidence stays structurally separated (§10, §29) so a future
  hand-off is possible, but nothing in v0.1 needs to call into calibration
  code. Confirmed by §4: "v0.1の必須依存にはしなくてよい."
  `masstrust` remains the intended downstream consumer of
  `CandidateAssessment`, not a dependency.
  `chematic` is a **direct Cargo dependency**, `risksieve`/`masstrust`
  are **not** — this is a Yellow (dependency-adding) decision, recorded
  here per Greenlane policy rather than left implicit.
- `veridict` is a candidate dev-dependency for the benchmark harness
  (§15/§27 Phase 6), not for the library itself.

## 4. Other Rust cheminformatics / mass-spec crates (non-sibling prior art)

- **`chemcore`** — lower-level molecule-graph primitives.
  [Depth-First survey](https://depth-first.com/articles/2020/01/20/cheminformatics-in-rust/)
- **`molrs`** — smaller-scope cheminformatics toolkit.
- **`openbabel`** — Rust FFI bindings to C++ OpenBabel (not pure Rust).
- **`mzdata`** — mzML/MS file-format reading.
  [crates.io/crates/mzdata](https://crates.io/crates/mzdata)
- **`rustyms`** — peptidoform handling, monoisotopic mass, spectrum
  annotation, MGF reading.
- **`mzcore`** (rusteomics org) — peptide-centric MS calculations around
  HUPO-PSI standards (mzML/MGF/USI).
  [github.com/rusteomics/mzcore](https://github.com/rusteomics/mzcore)

None are nucleoside/DNA-adduct-aware — proteomics-first or generic
small-molecule. Confirms no existing Rust crate combines nucleobase/
nucleoside chemistry semantics with MS/MS evidence scoring; that gap is
what Adductra fills.

## 5. First reference case selection

Colibactin (`AGENTS.md` §14's suggested candidate) is real and well
published (Balskus/Herzon groups): electrophilic-cyclopropane alkylation
forming adenine/diadenine cross-link adducts, elucidated via MS2/MS3 plus
¹³C-Cys/Met isotope feeding because the native metabolite can't be isolated
intact.
[Science 2019 aar7785](https://www.science.org/doi/10.1126/science.aar7785),
[Biochemistry 2019](https://pubs.acs.org/doi/10.1021/acs.biochem.8b01023),
[Science 2019 aax2685](https://pmc.ncbi.nlm.nih.gov/articles/PMC6820679/),
[Science 2025 ady3571](https://www.science.org/doi/10.1126/science.ady3571).

**Decision: defer colibactin, use it later as a "hard mode" case.** Its
cross-linked structures have no purchasable pure standard, and exact-mass /
fragment data is scattered across supplementary materials rather than one
reusable table — too much data-engineering risk for the first benchmark
corpus, which needs to validate the *evidence engine*, not chase down
literature data.

**v0.1 benchmark reference case: 8-oxo-dG / 8-oxo-Gua**, with
**aflatoxin B1–N7-guanine / FapyGua** as a second case once the engine is
stable. Rationale:

- 8-oxo-dG: `[M+H]⁺` m/z 168 (base), well-characterized diagnostic ions at
  m/z 140/112 (sequential CO loss); isotope-dilution LC-MS/MS standard
  exists (LOD ≈ 3.5 fmol); its known base/nucleoside in-source artifact is
  itself a useful evidence-consistency test case (missing vs. contradicting
  evidence, §3/§25).
  [ScienceDirect 2010](https://www.sciencedirect.com/science/article/abs/pii/S0891584909006558),
  [Nature Sci Rep 2016](https://www.nature.com/articles/srep32581)
- AFB1-N7-Gua / FapyGua: published MS2/MS3 CID schemes, ¹⁵N₅-labeled
  internal standards, in-vivo isotope-dilution quantification — directly
  exercises the isotope-shift evidence type (§7 P1).
  [ACS Omega 2023](https://pubs.acs.org/doi/10.1021/acsomega.3c01328),
  [NIST](https://www.nist.gov/publications/identification-and-quantification-aflatoxin-guanine-and-fapyguanine-adducts-mouse-liver)

Both have purchasable/well-documented standards and mass+fragment data
concentrated in a small number of papers, which keeps Phase 6 benchmark
construction tractable.

**Update (AFB1-N7-Gua / FapyGua case built, `tests/afb1_n7_gua_benchmark.rs`).**
A follow-up research pass verified this case in more depth before coding
it, and found the NIST link above is a thin 2018 SOT conference-abstract
record with no formula/m/z data on the page itself, not a paper — the
actual data (formulas, fragment m/z, ¹⁵N₅ labeling scheme) came from the
open-access full text of the ACS Omega 2023 paper
([PMC10134230](https://pmc.ncbi.nlm.nih.gov/articles/PMC10134230/)), with
a peer-reviewed corroborating (abstract-only verified) sibling paper:
Coskun et al., *Chem. Res. Toxicol.* 2019, 32(1):80-89,
[doi:10.1021/acs.chemrestox.8b00202](https://pubs.acs.org/doi/10.1021/acs.chemrestox.8b00202),
[PMID 30525498](https://pubmed.ncbi.nlm.nih.gov/30525498/). Formulas
(AFB1-N7-Gua: C22H17N5O8; AFB1-FapyGua: C22H19N5O9, i.e. N7-Gua + H2O)
cross-checked against PubChem CID 135625225 and CID 135854982, and all
fragment/precursor masses used in the rules and test fixture were
independently hand-computed from those formulas against
`mass_table`'s own constants — not copied from the paper's rounded
values — then confirmed to reproduce the paper's reported nominal m/z
(152, 329, 480, 498, 452) exactly. One structural caveat worth flagging:
the guanine N7 in this adduct is a quaternized, intrinsically
positively-charged imidazolium nitrogen, yet the field's own convention
(and the only arithmetic that reproduces the reported m/z) treats
C22H17N5O8 as the neutral "M" and the observed ion as ordinary
`[M+H]+` — Adductra follows that convention for consistency with the
literature it's checked against, not because the physical picture is
that simple.

## 6. Differentiation assessment

Existing infrastructure solves narrow slices: curated databases
(Frontiers/GitLab DB, DNA Adduct Portal) supply ground-truth masses,
formulas, and some real fragments; nLossFinder and DIA-based workflows solve
nontargeted screening. MutAIverse is the closest existing attempt at
end-to-end explainable adduct discovery but is a preprint platform, not an
installable typed library; CFM-ID/MetFrag/SIRIUS are metabolite-generic with
no nucleoside awareness. What's missing — and where Adductra adds real
value — is a small, typed, embeddable *evidence-combination and
explanation* layer: given a candidate adduct plus MS/MS observations,
produce a structured per-evidence-type breakdown (mass match, precursor
consistency, diagnostic ions, neutral losses, isotope shifts, nucleobase/
nucleoside classification, structural plausibility) with explicit
ranking-vs-confidence separation, instead of a peak-picking script or an
opaque end-to-end score. `chematic` already supplies molecular primitives,
`veridict` already supplies generic statistical benchmarking, and
`masstrust`/`risksieve` already supply generic calibration — so Adductra's
actual scope is the DNA/nucleoside-adduct domain model plus multi-evidence
explainable scoring that sits between `chematic`'s primitives and
`masstrust`'s calibration layer, composing existing infrastructure rather
than duplicating it. This directly satisfies `AGENTS.md`'s mandate to not be
"just a Rust reimplementation of existing software."
