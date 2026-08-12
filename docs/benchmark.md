# Benchmark

Implements the v0.1 milestone slice of `AGENTS.md` §15/§27 Phase 6.
Source: `tests/eight_oxo_dg_benchmark.rs`.

## Current fixture: 8-oxo-2'-deoxyguanosine

Reference case selection and rationale: `docs/landscape.md` §5. One
observation (`[M+H]+` precursor m/z 284.0989, product ions at
168.0516/140.0567/112.0618 — deoxyribose loss then two sequential CO
losses) evaluated against three candidates:

```text
8-oxo-dG            correct candidate, right formula, Guanine origin
adenine-isomer       same formula/mass, wrong nucleobase_origin tag
                     (Guanine-specific fragment rules don't fire on it)
mass-close-decoy     ~129 ppm off on exact mass (O-for-CH4 near-isobaric
                     swap) — genuinely wrong, not just structurally
                     different
```

This exercises, in one pass: exact-mass evidence, precursor-consistency
evidence (including the charge/polarity self-consistency check),
rule-driven diagnostic-fragment and neutral-loss evidence, the
`Missing`-vs-`Contradicting` distinction (see
`missing_ms2_data_still_ranks_on_mass_alone_without_false_contradiction`),
deterministic ranking, and structured + text explanation — the full
pipeline named in the "最初のゴール" milestone.

## Benchmark categories from §15 — coverage so far

```text
known positive adducts          done (8-oxo-dG)
decoy / competing candidates    done (adenine-isomer)
mass-close alternatives         done (mass-close-decoy)
missing-evidence cases          done (precursor-only observation test)
contradictory-evidence cases    partially (mass-close-decoy is
                                 mass/precursor-contradicted; no case yet
                                 with a genuinely present-but-wrong
                                 fragment peak)
```

## Not yet built

This is a single hand-written fixture, not the Phase 6 benchmark harness:
no `top-1 accuracy` / `top-k recall` / `MRR` / `candidate reduction` /
`ranking margin` / `evidence coverage` metrics computed across a corpus,
and no second reference case (AFB1-N7-Gua / FapyGua, per
`docs/landscape.md` §5) yet. Both are natural next steps once more rule
data exists to make a multi-case corpus meaningful — see `ROADMAP.md`.
