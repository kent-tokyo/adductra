# Benchmark

Implements the v0.1 milestone slice of `AGENTS.md` §15/§27 Phase 6.
Sources: `tests/eight_oxo_dg_benchmark.rs`, `tests/afb1_n7_gua_benchmark.rs`,
`tests/ethenoadenine_benchmark.rs`, `tests/o6_me_dg_benchmark.rs`,
`tests/n6_me_da_benchmark.rs`, `tests/n2_ethyl_dg_benchmark.rs`,
`tests/benchmark_corpus.rs`.

## Fixture 1: 8-oxo-2'-deoxyguanosine

Reference case selection and rationale: `docs/landscape.md` §5. One
observation (`[M+H]+` precursor m/z 284.0989, product ions at
168.0516/140.0567/112.0618 — deoxyribose loss then two sequential CO
losses) evaluated against three candidates:

```text
8-oxo-dG            correct candidate, right formula, Other("8-oxo-guanine")
adenine-isomer       same formula/mass, wrong nucleobase_origin tag
                     (the 8-oxo-specific fragment rules don't fire on it)
mass-close-decoy     ~129 ppm off on exact mass (O-for-CH4 near-isobaric
                     swap) — genuinely wrong, not just structurally
                     different
```

A fourth scenario (`present_but_wrong_fragment_peaks_lower_the_ranking_score`)
pairs the correct candidate with a spectrum showing peaks at the wrong
positions: fragment evidence must register as `Contradicting`, not
`Missing`, and the score must be lower than the matching-spectrum case.

This exercises, in one pass: exact-mass evidence, precursor-consistency
evidence (including the charge/polarity self-consistency check),
rule-driven diagnostic-fragment and neutral-loss evidence, the
`Missing`-vs-`Contradicting` distinction, isotope evidence (`NotApplicable`
when no label is used), deterministic ranking, structured + text
explanation, and `AdductReport` construction/round-trip — the full
pipeline named in the "最初のゴール" milestone.

## Fixture 2: AFB1-N7-guanine / AFB1-FapyGua

Reference case selection: `docs/landscape.md` §5 (updated with a deeper
verification pass before this fixture was built — see that section for
the full citation trail and the caveat about the guanine N7's actual
charge state). One observation matching AFB1-N7-Gua's published spectrum
(`[M+H]+` precursor m/z 480.114989, fragments at 152.056686 and
329.065579 — the complementary N7(Gua)-C8(AFB1) bond-cleavage pair)
evaluated against two real structures, not an invented decoy:

```text
AFB1-N7-Gua      correct candidate for this spectrum, C22H17N5O8
AFB1-FapyGua     a real, distinct in-vivo interconversion product
                 (N7-Gua + H2O, C22H19N5O9) — genuinely wrong for THIS
                 spectrum (~18 Da off), not a fabricated decoy
```

A second test (`fifteen_n5_labeled_guanine_shift_supported_by_isotope_evidence`)
exercises `IsotopeEvidenceEvaluator` against the paper's real ¹⁵N₅-labeled
internal standard (all 5 labeled nitrogens are guanine's — AFB1 itself
has none), including the boundary case of a 6th label being chemically
impossible for this formula.

**Discovered while building this fixture**: tagging AFB1-N7-Gua as
`NucleobaseOrigin::Guanine` (the "obviously correct" tag) caused the
8-oxo-dG-specific CO-loss rules — originally targeted at
`NucleobaseOrigin::Guanine` too — to incorrectly fire on it, since both
are guanine-derived. Fixed by retargeting those rules at
`NucleobaseOrigin::Other("8-oxo-guanine")` instead of the generic
`Guanine` variant (`rules/dna_adduct_fragments.json`,
`src/evidence/{mass,fragment}.rs` test fixtures,
`tests/eight_oxo_dg_benchmark.rs`) — a real instance of the "rule
generalizes to the wrong scope" failure `AGENTS.md` §7 warns against,
caught only because a second Guanine-tagged candidate was added. The
nucleoside-agnostic `"Any"`-targeted deoxyribose-loss rule still fires on
AFB1-N7-Gua (a base-level conjugate with no sugar) and correctly
contradicts — that one is a milder, already-documented scope limitation
(see `src/evidence/fragment.rs`'s module doc), not a correctness bug: its
"not observed" verdict is factually true for this candidate.

## Fixture 3: 1,N6-ethenoadenine (εdA) — the first non-guanine case

Reference case selection: `docs/landscape.md` §5 (added specifically to
stress-test generalization — both prior cases are guanine-derived).
Sourced from Cui et al., *Int J Environ Res Public Health* 2014,
11(10):10902–10914, doi:10.3390/ijerph111010902 (open text: PMC4211013).
One observation matching εdA's published spectrum (`[M+H]+` precursor
m/z 276.109116, single fragment at 160.061772 — deoxyribose loss to the
free-base ion) evaluated against a real, distinct, commonly co-measured
decoy:

```text
etheno-dA   correct candidate, C12H13N5O3, Adenine origin
etheno-dG   1,N2-etheno-2'-deoxyguanosine, C12H13N5O4, Guanine origin —
            real co-formed adduct, genuinely wrong mass for this
            spectrum (~16 Da off, one more oxygen)
```

**The point of this fixture is what it *doesn't* add**: zero new rule
data. The etheno bridge is fused entirely into the base (adenine's N1
and exocyclic N6), so the sugar/glycosidic bond is chemically untouched
— the pre-existing nucleobase-agnostic `nucleoside-deoxyribose-loss`
rule (`target: "Any"`, originally written for and verified against
8-oxo-dG) already covers εdA's one diagnostic transition, and does so
correctly: independently hand-computed at 116.047344 Da vs. the rule's
existing 116.0473 Da, matching to 4 significant figures. That's real
corroborating evidence the rule is generically correct, not merely
correct-by-coincidence for guanine chemistry — recorded in the rule's
own citation field (`rules/dna_adduct_fragments.json`, bumped to
v1.1.0).

A second test (`fifteen_n5_labeled_etheno_da_shift_supported_by_isotope_evidence`)
reuses the same ¹⁵N₅/6th-label-impossible pattern as the AFB1 fixture —
adenine's ring also has exactly 5 nitrogens.

**v0.2.2 cross-validation**: `spectral_library_match_cross_validated_against_real_la_barbera_8_oxo_dg_spectrum`
(Fixture 1) and `spectral_library_match_cross_validated_against_real_la_barbera_etheno_da_spectrum`
(this fixture) each add a *second*, independently-sourced
`ReferenceSpectrum` — real experimental peaks from La Barbera et al.
2022 (see Fixtures 4–6 below for the full citation), not copied from
this fixture's own hand-derived values. Both land within ~0.0005 Da of
the pre-existing peaks and score cosine > 0.98, a genuine external
corroboration of data that was already independently verified, not just
a second test of the same numbers.

## Fixtures 4–6: O6-Me-dG, N6-Me-dA, N2-Ethyl-dG — corpus growth from an external database

Source: La Barbera G, Nommesen KD, Cuparencu C, Stanstrup J, Dragsted LO
(2022), "A Comprehensive Database for DNA Adductomics," *Frontiers in
Chemistry* 10:908572, doi:10.3389/fchem.2022.908572. Published under
Frontiers' CC BY 4.0 policy; the paper's Data Availability Statement
names `gitlab.com/nexs-metabolomics/projects/dna_adductomics_database`
as the deposit — no separate LICENSE file exists there, but this is the
same basis `ROADMAP.md` recorded when the database was first evaluated
(post-v0.2.0) and deliberately deferred.

The database's `_input/MS MS spectra standards.xlsx` contains 15 sheets
of **real experimental** LC-MS/MS standards (4 collision energies each)
— distinct from, and higher-value than, the database's separate 580
CFM-ID *predicted* spectra. Two of the 15 (8-oxo-dG, "1,N6-etheno-dA")
were already in Adductra's corpus and used for cross-validation above.
Of the other 13, 9 resolve cleanly against the database's master
compound table (formula + mass + DOI + InChIKey all present); this
round adds 3 of those 9 as new benchmark cases — a small first batch,
not the full 9 (see `ROADMAP.md` v0.2.2 for what's deliberately
deferred). Each new case:

```text
O6-Me-dG      C11H15N5O4, [M+H]+ 282.119680, vs. decoy 1-Me-dG (same formula)
N6-Me-dA      C11H15N5O3, [M+H]+ 266.124766, vs. decoy 2-Me-dA (same formula)
N2-Ethyl-dG   C12H17N5O4, [M+H]+ 296.135331, vs. decoy O6-Ethyl-dG (same formula)
```

All masses independently hand-computed from the formula against
`mass_table`'s constants (not copied from the database — see each
fixture's module doc). All three share the identical deoxyribose-loss
delta, 116.047344 Da, independently confirmed for a fourth and fifth
time beyond the etheno-dA fixture. Zero new fragment-rule data: the
pre-existing `Any`-targeted `nucleoside-deoxyribose-loss` rule already
covers all three. Each diagnostic fragment's *observed* value used for
spectral-library matching is the database's real measured peak (e.g.
166.07292 for O6-Me-dG), distinct from the *theoretical* value used for
the `Observation`/rule check — a deliberate real-data-vs-theory split,
not an inconsistency.

**A genuine, documented limitation, not a fabricated edge case**: all
three decoys are real, distinct regioisomers with the *exact same
formula* as the correct candidate (1-Me-dG vs. O6-Me-dG, etc.). Unlike
Fixture 1's `adenine-isomer` decoy — discriminated by a
`NucleobaseOrigin`-targeted rule — `nucleoside-deoxyribose-loss` is
`Any`-targeted and computed purely from the observed precursor/fragment
delta; it doesn't inspect candidate structure at all, so it fires
identically for a candidate and its same-formula regioisomer. Each new
fixture file has a dedicated test asserting this tie explicitly
(`..._tie_on_mass_and_generic_rule_evidence_alone`) rather than hiding
it. The source database has no reference spectrum for any of the three
decoys either, so `SpectralLibraryEvidenceEvaluator` can't resolve the
tie here — it correctly abstains (`NotApplicable`) on the decoy instead
of guessing, which is a real and valuable property, but distinguishing
same-formula regioisomers in general still requires a per-isomer
reference spectrum this round doesn't have for every isomer. See
`ROADMAP.md` v0.2.2 for this as recorded future work.

## Benchmark categories from §15 — coverage so far

```text
known positive adducts          done (8-oxo-dG, AFB1-N7-Gua, etheno-dA)
decoy / competing candidates    done (adenine-isomer; AFB1-FapyGua and
                                 etheno-dG as real-structure "decoys"
                                 for the wrong observation)
mass-close alternatives         done (mass-close-decoy)
missing-evidence cases          done (precursor-only observation test)
contradictory-evidence cases    done (present-but-wrong fragment peaks;
                                 mass-close-decoy is mass/precursor-
                                 contradicted)
non-guanine generalization      done (etheno-dA — different nucleobase,
                                 zero new rule data needed)
```

## Corpus metrics (`tests/benchmark_corpus.rs`)

All six fixtures above are re-assembled (small, independent copies of
their observation/candidate setup — see that file's module doc for why
it doesn't import from the fixture files) into a 6-case corpus and
scored against §15's metric list, using the same evaluator set as the
CLI's default (mass, fragment, isotope — no `--reference-spectra`):

```text
top_1_accuracy=0.67  top_2_recall=1.00  mrr=0.83
mean_margin=6.67  mean_evidence_coverage=0.78  candidate_reduction=0.23
```

(Numbers as of this writing — see the test's own `println!` output for
current values; `cargo test --test benchmark_corpus -- --nocapture` to
see them per-case.) `top_1_accuracy`/`mrr` dropped from 1.00 in the
3-case v0.1 corpus because 3 of the 6 cases (O6-Me-dG, N6-Me-dA,
N2-Ethyl-dG — see Fixtures 4–6 above) are known, documented ties with
their same-formula decoy under this evaluator set, not a regression —
`corpus_metrics_meet_v01_baseline` asserts the tie is *exact* for those
three (`margin == 0.0`) and *strict* for the original three
(`margin > 0.0`), so either direction of drift fails CI.
`top_2_recall == 1.0` is the honest replacement for the old "every known
adduct ranks first" guarantee: every correct candidate is still never
outranked by its decoy, even in a tie. `candidate_reduction` is defined
here as the fraction of all candidates, across every case, with
`ranking_score <= 0` — i.e. how much of the candidate set the evidence
engine net-excludes.

Not using `veridict` for this (`docs/landscape.md` §3 earmarks it for
exactly this role) — its real API reads as built for statistically
comparing two ranking *configurations* across many trials (win-rate,
bootstrap CI, SPRT), which isn't the shape of "compute MRR over a
handful of known-answer cases." Worth revisiting once there's an actual
A-vs-B ranking comparison to run (e.g. comparing two `Ranker` weight
configurations).

## Not yet built

Six reference cases — still a small corpus relative to the field (see
`ROADMAP.md` v0.2.2 for the external-review context that motivated
Fixtures 4–6). Concretely open:

- **Same-formula regioisomer discrimination** — the gap Fixtures 4–6
  document explicitly: `nucleoside-deoxyribose-loss` can't tell same-
  formula isomers apart, and none of the three decoys used this round
  have their own reference spectrum in the source database. Adding a
  per-isomer spectrum (from the database's other sheets, or elsewhere)
  once one is genuinely available would resolve a specific, already-
  identified tie rather than being speculative future work.
- The other 6 cleanly-resolving La Barbera standards not used this
  round (ABP-dG, etheno-dC, M1-dG, 6-oxo-M1-dG, C8-MeIQx-dG, 5-Me-dC)
  and the database's 580 CFM-ID *predicted* spectra (lower value than
  the experimental standards used here — see `ROADMAP.md`).
- A genuinely cross-linked/multi-nucleobase adduct (colibactin, deferred
  as "hard mode" in `docs/landscape.md` §5).
