# Scoring

Implements `AGENTS.md` §10, §29. Source: `src/ranking.rs`.

## Ranking score is not confidence

`Ranker::score` sums `±(kind_weight × strength_weight)` across a
candidate's `Evidence`:

```text
strength_weight: Weak = 1.0, Moderate = 2.0, Strong = 3.0
direction:       Supporting → +magnitude, Contradicting → -magnitude
                 Missing / Unavailable / NotApplicable → 0 (no signal)
```

`kind_weight` defaults to `1.0` per `EvidenceKind` and is overridable via
`Ranker::with_kind_weight` (e.g. to weight isotope evidence higher when a
label was deliberately used). The result is an unbounded, transparent
`f64` — comparable *within one report*, never a probability, and never
comparable across reports or models. There is no `confidence` field
anywhere in v0.1's public API; calibration is explicitly out of scope
(`AGENTS.md` §4, §10) and is the intended job of the sibling `masstrust`
crate, downstream of `AdductReport`.

## Why weighted-sum, not a model

Per §10: "最初から複雑なMLモデルを入れない" — start with transparent,
rule-based aggregation. `Ranker` is the whole ranking algorithm; there is
no hidden normalization, no learned weights, and every contribution to
the final score is individually inspectable via
`CandidateAssessment::evidence`.

## Determinism

`Ranker::rank` sorts by score descending using `f64::total_cmp` (never
`partial_cmp().unwrap()`, which panics on NaN and leaves ordering
ambiguous when scores tie) and breaks ties on `candidate_id` ascending.
Given the same evidence, the same ranking always comes out — see the
`ranking_is_deterministic_and_sorted` property test in
`tests/properties.rs`.

## Explanation is structured first, text second

`explain(&CandidateAssessment) -> Explanation` builds one
`ExplanationLine` per evidence item (with a `+`/`-`/`·` polarity and a
rendered clause specific to that evidence's `EvidenceDetail` variant).
`Explanation` itself is the first-class, serializable representation
(`AGENTS.md` §11); `Explanation::to_text()` is one rendering of it, not
the source of truth.
