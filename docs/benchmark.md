# Benchmark

Implements the v0.1 milestone slice of `AGENTS.md` §15/§27 Phase 6.
Sources: `tests/eight_oxo_dg_benchmark.rs`, `tests/afb1_n7_gua_benchmark.rs`,
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

## Benchmark categories from §15 — coverage so far

```text
known positive adducts          done (8-oxo-dG, AFB1-N7-Gua)
decoy / competing candidates    done (adenine-isomer; AFB1-FapyGua as a
                                 real-structure "decoy" for the wrong obs)
mass-close alternatives         done (mass-close-decoy)
missing-evidence cases          done (precursor-only observation test)
contradictory-evidence cases    done (present-but-wrong fragment peaks;
                                 mass-close-decoy is mass/precursor-
                                 contradicted)
```

## Corpus metrics (`tests/benchmark_corpus.rs`)

Both fixtures above are re-assembled (small, independent copies of their
observation/candidate setup — see that file's module doc for why it
doesn't import from the two files above) into a 2-case corpus and scored
against §15's metric list:

```text
top_1_accuracy=1.00  top_2_recall=1.00  mrr=1.00
mean_margin=14.00  mean_evidence_coverage=0.83  candidate_reduction=0.40
```

(Numbers as of this writing — see the test's own `println!` output for
current values; `cargo test --test benchmark_corpus -- --nocapture` to
see them per-case.) `candidate_reduction` is defined here as the fraction
of all candidates, across every case, with `ranking_score <= 0` — i.e.
how much of the candidate set the evidence engine net-excludes.
`corpus_metrics_meet_v01_baseline` asserts these stay at their current
(already-correct) values, so a future regression in either fixture's
ranking fails CI rather than silently drifting.

Not using `veridict` for this (`docs/landscape.md` §3 earmarks it for
exactly this role) — its real API reads as built for statistically
comparing two ranking *configurations* across many trials (win-rate,
bootstrap CI, SPRT), which isn't the shape of "compute MRR over 2
known-answer cases." Worth revisiting once there's an actual A-vs-B
ranking comparison to run (e.g. comparing two `Ranker` weight
configurations).

## Not yet built

Only 2 reference cases — a third (independent of guanine-derived
adducts) would meaningfully stress-test whether the evidence model
generalizes, and would give the corpus metrics above more than 2 data
points to be meaningful over.
