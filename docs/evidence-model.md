# Evidence Model

Implements `AGENTS.md` §5–§6, §25–§26. Source: `src/model/evidence.rs`,
`src/model/observation.rs`, `src/model/candidate.rs`,
`src/model/assessment.rs`.

## Core types

```text
Observation          what was measured (precursor m/z, charge, ion adduct,
                      product ions, isotope labels, optional formula hint)
AdductCandidate       a candidate under evaluation (always has a resolvable
                      formula; SMILES optional)
Evidence              one test: what was tested, a typed detail payload,
                      direction, strength, source, method, provenance
EvidenceSet           an ordered collection of Evidence for one candidate
CandidateAssessment   candidate_id + EvidenceSet + ranking_score
AdductReport          observation_id + ranked CandidateAssessments + provenance
```

There is deliberately no `f64 score` field anywhere in `Evidence` — every
evaluator returns typed [`EvidenceDetail`] variants (`Mass`,
`PrecursorConsistency`, `DiagnosticFragment`, `NeutralLoss`,
`IsotopeLabel`, `Generic`) carrying expected/observed/delta/tolerance.

## Direction, strength, and the absence-of-evidence rule

`EvidenceDirection` has five values, not two:

```text
Supporting       observation favors this candidate
Contradicting    observation is inconsistent with this candidate
Missing          expected to be observable, wasn't — see MissingReason
Unavailable      this evidence type couldn't be evaluated (unrelated to
                 the candidate, e.g. required observation not collected)
NotApplicable    this evidence type doesn't apply here at all
```

`EvidenceStrength` (`Weak`/`Moderate`/`Strong`) is present **iff**
direction is `Supporting` or `Contradicting`; `MissingReason` is present
**iff** direction is `Missing`. This is enforced by construction —
`Evidence`'s fields are private, built only through
`Evidence::supporting`/`contradicting`/`missing`/`unavailable`/
`not_applicable`, and the `Deserialize` impl (via `TryFrom<RawEvidence>`)
re-validates the same invariant so a hand-crafted or externally-supplied
JSON payload can't smuggle in an inconsistent combination.

**The sharp edge (`AGENTS.md` §25): "measured but absent" is
`Contradicting`, not `Missing`.** `Missing` means the evaluator couldn't
even test the hypothesis (no MS2 acquired at all). If MS2 *was* acquired
and the expected fragment genuinely isn't there, that counts against the
candidate — see `FragmentEvidenceEvaluator::build_evidence` in
`src/evidence/fragment.rs`, which branches on `has_ms2` for exactly this
reason. `MissingReason` still distinguishes `NotMeasured` /
`BelowThreshold` / `OutsideAcquisitionRange` / `MeasuredButAbsent` for
evaluators that want a genuinely ambiguous "we looked but can't be sure"
case; v0.1's own evaluators only ever emit `NotMeasured`, since
`Observation` doesn't yet carry acquisition-range/intensity-threshold
metadata to distinguish the other three (see `ROADMAP.md` discovered
work).

## Source, not just direction

`EvidenceSource` (`Experimental` / `Literature` / `Rule` / `Database` /
`Derived` / `Predicted` / `UserProvided`) travels with every `Evidence`
so a consumer can tell "this was measured" from "this was computed from a
literature rule" from "this was predicted" — `AGENTS.md` §26 forbids
presenting these as equivalent.
