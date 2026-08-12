# Adductra 開発指示書

## 目的

新しいRustライブラリ **Adductra** を開発する。

Adductraの中核コンセプトは、

> **Evidence-first DNA adduct identification and analysis**

とする。

未知または既知のDNA adductについて、単一のスコアだけを返すのではなく、

* 分子構造
* exact mass / molecular formula
* MS/MS fragmentation
* neutral loss
* nucleobase / nucleoside由来の証拠
* isotope labeling
* precursor/product ion consistency
* structural plausibility
* provenance

などの複数の証拠を統合し、

**「なぜこのDNA adduct候補が支持されるのか」**

を機械可読かつ人間可読な形で説明できるライブラリにする。

研究用途を第一とし、臨床診断ソフトを目指さない。

---

# 1. 最初に行うこと

実装前に必ず以下を行うこと。

1. DNA adductomics / adduct identification分野の既存ツール、データベース、論文を調査する。
2. GitHub / crates.io / PyPI等で競合・類似実装を調査する。
3. 特に以下を確認する。

   * DNA adduct MS/MS identification
   * adductomics
   * modified nucleosides / nucleobases
   * LC-MS/MS adduct annotation
   * isotope-label assisted identification
   * DNA damage databases
4. 「既に十分成熟した実装が存在する機能」と「Adductraだから価値が出る部分」を区別する。
5. 調査結果を `docs/landscape.md` にまとめる。

既存ソフトの単なるRust再実装にはしないこと。

Adductraの差別化は、

* evidence-first
* explainable
* reproducible
* typed
* modular
* uncertainty-aware
* Rust-native

に置く。

---

# 2. Adductraの責務

Adductraは基本的に、

```text
observations
     ↓
candidate generation
     ↓
evidence extraction
     ↓
evidence aggregation
     ↓
candidate ranking
     ↓
explanation
```

を担当する。

概念的には以下。

```text
          DNA adduct experiment

 precursor m/z
 product ions
 molecular formula
 isotope labels
 optional structure hints
        │
        ▼
   ┌─────────────┐
   │  Adductra   │
   └──────┬──────┘
          │
    candidate adducts
          │
          ▼
   evidence evaluation
          │
     ┌────┼────┐
     ▼    ▼    ▼
   mass  MS/MS isotope
     │    │    │
     └────┼────┘
          ▼
     ranked evidence
          │
          ▼
   explanation report
```

---

# 3. Adductraが担当しないもの

v0.1では以下をスコープ外とする。

* 癌診断
* 患者のrisk prediction
* clinical decision support
* 発癌因果関係の断定
* mutational signature decomposition
* 大規模proteomics/metabolomics pipeline
* LCピーク検出そのもの
* raw vendor format parser
* chromatographic alignment
* deep-learning spectrum predictor
* full retrosynthesis
* 一般的な分子構造ライブラリの再実装
* calibration / selective predictionの独自再実装

特に、

```text
chemical exposure
      ↓
DNA adduct
      ↓
mutation
      ↓
cancer
```

という将来的な接続を想定するが、

**v0.1ではDNA adduct evidenceまでを責務とする。**

---

# 4. 既存Rustライブラリとの関係

既存ライブラリをコピーせず、責務を分離する。

## chematic

分子構造処理は可能な限り `chematic` を利用する方向で検討する。

候補：

* molecular representation
* SMILES / SMARTS
* formula
* exact massに必要な構造情報
* substructure
* stereochemistry
* molecular graph
* structure I/O

ただしAdductraをchematicに強結合させる前に、実際の公開APIと現在のcrate状態を確認すること。

必要なら薄いadapter layerを作る。

Adductra内部に独自cheminformatics engineを増殖させない。

## masstrust

Adductraとmasstrustの責務は明確に分ける。

Adductra：

```text
candidate generation
evidence extraction
evidence score
candidate ranking
```

masstrust：

```text
confidence
risk-coverage
abstention
selective prediction
```

つまり、

```text
Adductra
   ↓
candidate ranking
   ↓
masstrust
   ↓
confidence / abstain
```

という統合を可能にする。

Adductra自身のranking scoreを「確率」や「confidence」と偽装しないこと。

## risksieve

必要になった段階で、

* calibration
* selective prediction
* risk-controlled acceptance

を接続可能にする。

v0.1の必須依存にはしなくてよい。

## veridict

benchmarkやcandidate-ranking手法の比較、統計評価に利用可能か調査する。

Adductra本体へ直接埋め込む必要はない。

---

# 5. 最初のデータモデル

最初にデータモデルを慎重に設計する。

最低限、以下に相当する概念を持つこと。

```rust
AdductCandidate
Observation
Evidence
EvidenceKind
EvidenceDirection
EvidenceStrength
EvidenceSource
EvidenceSet
CandidateAssessment
AdductReport
Provenance
```

具体的な型名は実装前の設計調査で改善してよい。

重要なのは、

**単一の `f64 score` に全情報を潰さないこと。**

例：

```text
Candidate:
  N2-guanine-derived adduct

Evidence:
  precursor_mass:
      observed: ...
      expected: ...
      delta_ppm: ...
      support: strong

  diagnostic_fragment:
      ion: ...
      observed: true
      support: strong

  neutral_loss:
      expected: ...
      observed: ...
      support: moderate

  isotope_label:
      expected_shift: ...
      observed_shift: ...
      support: strong

  missing_fragment:
      expected: ...
      observed: false
      contradiction: weak
```

positive evidenceだけでなく、

* supporting
* contradicting
* missing
* unavailable
* not-applicable

を区別できる設計にする。

---

# 6. Evidenceの設計原則

Evidenceには可能な限り以下を保存する。

```text
what was tested
what was expected
what was observed
difference
tolerance
support / contradiction
source
method
provenance
```

単なる、

```text
score = 0.87
```

は禁止。

少なくとも内部では説明を復元できること。

---

# 7. v0.1で優先するEvidence

## P0

### Exact mass

* theoretical mass
* observed mass
* ppm error
* configurable tolerance
* formula consistency

### Precursor consistency

* charge
* ionization assumptions
* adduct ion type
* precursor m/z

### Diagnostic fragments

known / expected fragmentとの一致。

### Neutral losses

DNA adductで重要な、

* base loss
* sugar-related loss
* nucleoside-related fragmentation

等を表現可能にする。

特定化合物だけにhard-codeせず、rule/data drivenにすること。

### Evidence aggregation

複数EvidenceをまとめてCandidateAssessmentを生成する。

---

## P1

### Isotope labeling

例：

```text
13C
15N
D
18O
```

について、

* expected shift
* observed shift
* label count
* compatible / incompatible

を扱えるようにする。

### Nucleobase / nucleoside classification

少なくとも、

* adenine
* guanine
* cytosine
* thymine
* uracil
* nucleoside-derived
* nucleotide-derived

などを拡張可能な形で扱う。

---

# 8. Candidate generation

candidate generationは一つの巨大アルゴリズムにしない。

trait/interfaceとして分離する。

概念：

```rust
trait CandidateGenerator {
    fn generate(
        &self,
        observation: &Observation,
    ) -> Result<Vec<AdductCandidate>, Error>;
}
```

候補generatorとして将来的に、

* exact-mass lookup
* formula constrained
* database-backed
* structure transformation
* user supplied
* reaction-rule based

などを追加できる構造にする。

v0.1では最小限、

1. user-supplied candidates
2. exact-mass constrained candidates

を優先する。

---

# 9. Evidence scorerを交換可能にする

Evidence評価もモジュール化する。

概念的には、

```rust
trait EvidenceEvaluator {
    fn evaluate(
        &self,
        observation: &Observation,
        candidate: &AdductCandidate,
    ) -> Result<Vec<Evidence>, Error>;
}
```

として、

```text
MassEvidence
FragmentEvidence
NeutralLossEvidence
IsotopeEvidence
StructureEvidence
```

を独立させる。

新しい論文や実験手法が出た際に、既存ロジックを壊さずEvidenceを追加できることを重視する。

---

# 10. Ranking

最初から複雑なMLモデルを入れない。

まずは、

* transparent weighted evidence
* rule-based aggregation
* explicit contradictions
* configurable weights

などでよい。

ただしAPIは後から、

```text
Bayesian model
learned ranker
likelihood model
```

などを追加できる構造にしておく。

重要：

**ranking scoreとconfidenceを区別すること。**

例えば、

```text
Candidate A
ranking_score = 12.4
```

は許されるが、

```text
confidence = 0.94
```

はcalibrationされていなければ返さない。

---

# 11. Explanation

Adductraの最重要機能の一つ。

最低限、

```rust
assessment.explain()
```

あるいは相当するAPIから、

```text
Candidate A ranked first because:

+ precursor mass matched within 1.8 ppm
+ diagnostic guanine-derived fragment observed
+ expected neutral loss observed
+ isotope shift matched two labelled nitrogens
- one expected low-intensity fragment was absent
```

のような説明を生成可能にする。

ただしhuman-readable textだけでなく、

**structured explanationを一次表現とする。**

JSON等へserializationできること。

---

# 12. Provenance

科学用途なのでprovenanceを最初から入れる。

最低限、

* rule version
* database version
* source citation identifier
* algorithm version
* parameter values
* tolerance
* software version

を追跡できる設計を検討する。

同じ入力と同じversion/parameterなら再現可能であること。

---

# 13. Rule / knowledge database

fragment ruleやknown adduct informationをコードへ大量にhard-codeしない。

可能なら、

```text
data/
rules/
references/
```

などとして機械可読データ化する。

各ruleに、

```text
id
description
target
expected observation
source
citation
version
```

を持たせる。

文献由来ルールとheuristicルールを区別する。

---

# 14. 最初のreference case

最初の研究上のreference caseとして、

**colibactin関連DNA damage / adduct evidence**

を有力候補として調査する。

ただし最初からcolibactin専用ライブラリにはしない。

目的は、

> 一般的なDNA adduct evidence engineが、実際の癌研究テーマで使えるか

を検証するためのreference use caseとすること。

文献・公開データが十分でなければ、より再現しやすいwell-characterized DNA adductを先にbenchmarkへ使用してよい。

選定理由を記録すること。

---

# 15. Benchmark corpus

v0.1前に小さくてもよいのでbenchmark corpusを作る。

最低限、

```text
known positive adducts
decoy / competing candidates
mass-close alternatives
missing-evidence cases
contradictory-evidence cases
```

を含める。

評価指標例：

```text
top-1 accuracy
top-k recall
MRR
candidate reduction
ranking margin
evidence coverage
contradiction detection
```

confidence calibrationはAdductra本体のranking評価と混同しない。

---

# 16. Property-based testing

化学計算・質量計算ではproperty testを積極的に使う。

候補：

* serialization round-trip
* candidate ordering determinism
* mass tolerance boundary
* ppm symmetry/definition
* isotope shift consistency
* empty evidence
* duplicate evidence
* NaN / inf rejection
* malformed observations

Rustらしく不正状態を型で可能な限り排除する。

---

# 17. Numerical robustness

`f64`を無条件に信用しない。

特に、

```text
NaN
±inf
negative intensity
negative tolerance
invalid charge
zero mass
impossible isotope count
```

などは明示的に処理する。

JSON serialization時の非有限値にも注意する。

---

# 18. CLI

v0.1で小さなCLIを用意することを検討する。

例：

```bash
adductra rank \
    --spectrum sample.mgf \
    --candidates candidates.jsonl
```

出力：

```text
Rank  Candidate            Score
1     candidate-A          ...
2     candidate-B          ...
3     candidate-C          ...
```

さらに、

```bash
adductra explain ...
```

またはJSON output。

CLIはライブラリAPIの薄いwrapperにする。

---

# 19. Python

Rust coreを安定させた後、

```text
Python bindings
```

を検討する。

ただし初期実装でRust APIを犠牲にしてPython都合の設計をしない。

Python利用者が多い研究領域なので、最終的には重要。

---

# 20. WASM

WASMは初期必須ではない。

coreが、

* file system
* threads
* native-only libraries

へ不必要に依存しない設計なら将来WASM化しやすい。

---

# 21. Documentation

最低限以下を書く。

```text
README.md
ARCHITECTURE.md
docs/landscape.md
docs/evidence-model.md
docs/scoring.md
docs/provenance.md
docs/benchmark.md
```

README冒頭で一文で説明する。

候補：

> Adductra is an evidence-first Rust toolkit for identifying and explaining DNA adduct candidates from mass-spectrometric and structural evidence.

さらに、

```text
Adductra is a research tool.
It does not diagnose cancer or establish causal exposure.
```

を明示する。

---

# 22. READMEに入れる概念図

```text
                   Adductra

       experimental observations
                 │
                 ▼
        candidate generation
                 │
                 ▼
       ┌─────────────────┐
       │ evidence engine │
       ├─────────────────┤
       │ exact mass      │
       │ MS/MS           │
       │ neutral losses  │
       │ isotope labels  │
       │ structure       │
       └────────┬────────┘
                │
                ▼
          candidate rank
                │
                ▼
           explanation
                │
          ┌─────┴─────┐
          ▼           ▼
      researcher   masstrust
                       │
                       ▼
                calibrated trust
```

---

# 23. Error handling

library codeで、

```rust
unwrap()
expect()
panic!()
```

に依存しない。

recoverableな入力エラーはResultで返す。

エラー型を公開APIとして設計する。

---

# 24. 性能

最初から過剰最適化しない。

ただしbenchmark harnessを早めに作り、

* 100 candidates
* 1,000 candidates
* 10,000 candidates

程度のcandidate rankingについて測れるようにする。

allocationやcloneを不必要に増やさない。

必要なら後からparallel evaluatorを追加する。

---

# 25. Scientific correctness

便利さより科学的正確性を優先する。

特に、

```text
absence of evidence
```

と

```text
evidence of absence
```

を混同しない。

未観測fragmentについても、

```text
not measured
below threshold
outside acquisition range
measured but absent
```

を可能なら区別する。

---

# 26. Evidence source

Evidenceには可能な限り、

```text
Experimental
Literature
Rule
Database
Derived
Predicted
UserProvided
```

等のsource categoryを持たせる。

予測由来Evidenceと実験由来Evidenceを同列に見せない。

---

# 27. 開発順序

## Phase 0 — Landscape / design

* competitor survey
* literature survey
* data-source survey
* architecture
* API sketch
* benchmark candidate selection

コードを書き始める前に設計メモを残す。

## Phase 1 — Core model

* observation
* candidate
* evidence
* provenance
* assessment
* serialization
* errors

## Phase 2 — Mass evidence

* formula
* monoisotopic mass
* ppm
* tolerance
* precursor matching

## Phase 3 — Fragment evidence

* product ions
* diagnostic fragments
* neutral losses
* rule representation

## Phase 4 — Candidate ranking

* deterministic transparent baseline
* explanation
* contradiction handling

## Phase 5 — Isotope evidence

* label representation
* expected shifts
* observed consistency

## Phase 6 — Benchmark

* real/reference cases
* decoys
* top-k metrics
* regression fixtures

## Phase 7 — Ecosystem integration

* chematic adapter
* masstrust hand-off format
* optional veridict/risksieve evaluation

## Phase 8 — CLI / docs / release

* CLI
* examples
* benchmark report
* crates.io metadata
* v0.1.0 readiness review

---

# 28. 将来ロードマップ

v0.1には入れないが、設計上は将来以下へ拡張可能にする。

```text
DNA adduct
   ↓
chemical exposure evidence
   ↓
DNA lesion
   ↓
mutation
   ↓
mutational signature
```

将来的な候補：

* DNA adduct databases
* metabolic activation evidence
* chemical exposure linking
* fragment prediction
* isotope-assisted untargeted adductomics
* LC retention evidence
* multi-stage MS
* mutational signature bridge
* cancer-specific research workflows

ただしこれらのためにv0.1を巨大化させない。

---

# 29. 設計上の最重要原則

Adductraは、

> **“Which candidate scored highest?”**

だけに答えるライブラリにしない。

必ず、

> **“What evidence supports or contradicts this candidate, and where did that evidence come from?”**

に答えられるようにする。

これがAdductraの存在理由。

---

# 30. 自律開発ルール

可能な範囲では自律的に開発を進めること。

以下は逐次確認不要。

* repository investigation
* literature / competitor survey
* tests
* benchmarks
* refactoring
* documentation
* CI improvement
* bug fixes
* small API refinements
* internal modularization

ただし以下は承認待ちにする。

* repositoryの公開
* crates.io / PyPI publish
* breaking API decisionで複数の合理的選択肢がある場合
* 他repoへのbreaking change
* external serviceへのcredential設定
* 大規模なscope変更
* 臨床用途を示唆する機能・表現

承認待ち事項が発生しても、独立して進められる作業は止めずに継続する。

---

# 31. 完了報告

各開発ラウンドの最後に、

```text
Completed
Evidence
Benchmarks
Tests
Open questions
Next actions
Approval needed
```

を簡潔に報告する。

「実装した」だけではなく、

```text
何を検証したか
どのbenchmarkが改善したか
何がまだ不明か
```

を明確にする。

---

# 最初のゴール

最初のマイルストーンは、

**既知DNA adduct + competing decoysを入力すると、exact mass / MS/MS / neutral-loss evidenceを構造化して評価し、候補を順位付けし、その順位の根拠を説明できる**

ところまで。

ここをv0.1の核とする。

最初から癌全体を解こうとしない。

まず、

> **DNA adduct identificationを、説明可能で再現可能なevidence problemとしてきれいに解く。**

そこからAdductraを育てること。
