# Open Word Book: Flashcard & Spaced Repetition Algorithm Specification

## 1. Overview & Objective

本ドキュメントは、Open Word Bookにおけるフラッシュカード学習アルゴリズムとデータベース設計の仕様を定義する。
従来の固定スケジュール型SRS（Spaced Repetition System）の複雑さを排除し、「指数移動平均（EMA）によるステートレスな学習追跡」と「コーパス頻度に基づく確率的サンプリング」を組み合わせたモダンなアプローチを採用する。

※注意: 本アプリの語彙マスタDB (`words.sqlite3`) はRead-Onlyであるため、本ドキュメントで定義する学習ステートは別の書き込み用DB（例: `user.sqlite3`）に保存し、JOINまたはRust側で統合して処理すること。

---

## 2. Database Schema

ユーザーの学習進捗と設定を記録する新しいSQLite構成（`user.sqlite3`を想定）。

### 2.1. `user_learning_states` テーブル

各単語の学習ステートを差分更新（オンラインアルゴリズム）で保持する。

| Column Name | Type | Description | Default / Init |
| :--- | :--- | :--- | :--- |
| `word_id` | INTEGER PK | `words.sqlite3` の `words.id` への参照 | - |
| `score_ema` | REAL | 定着度スコアの指数移動平均 | - |
| `variance_ema` | REAL | 定着度スコアの指数平滑移動分散 (EWMV) | - |
| `last_reviewed_at`| INTEGER | 最終学習日時（UNIX Timestamp Sec） | - |
| `review_count` | INTEGER | これまでに学習（回答）した回数 | `0` |
| `is_ignored` | BOOLEAN | ユーザーが「既知」として除外したか (0 or 1) | `0` (false) |

### 2.2. `user_learning_settings` テーブル (Optional / またはRust Config)

ハイパーパラメータ。将来的にユーザーがUIから調整できるようにKey-Valueストアまたは1行のレコードとして保持。

| Key | Default Value | Description |
| :--- | :--- | :--- |
| `alpha` | `0.3` | EMAの更新係数 (0.0 ~ 1.0)。大きいほど直近の回答を重視。 |
| `weight_mean` | `1.0` | 苦手度（スコアの低さ）を重視する係数 |
| `weight_variance`| `1.0` | 不安定度（分散の大きさ）を重視する係数 |
| `time_decay_factor`| `0.00001` | 経過秒数に乗算する係数（約1日でウェイト+0.8程） |

---

## 3. Core Algorithm (Phase 1)

### 3.1. 状態の更新ロジック (State Update)

フラッシュカードでユーザーが回答（Score: 0=全くわからない, 1=思い出すのに時間がかかった, 2=簡単にわかった）するたびに、対象単語のレコードを以下の計算式で**UPDATE（またはUPSERT）**する。

**[Rustでの実装フロー]**

1.  現在のステートを取得。レコードが存在しなければ `count = 0` として初期化。
2.  計算処理:

```rust
let alpha = 0.3; // settingsから取得

if count == 0 {
    new_score_ema = current_score as f32;
    new_variance_ema = 0.0;
} else {
    let diff = current_score as f32 - old_score_ema;
    new_score_ema = old_score_ema + alpha * diff;
    new_variance_ema = (1.0 - alpha) * (old_variance_ema + alpha * diff.powi(2));
}

let new_count = old_count + 1;
let new_last_reviewed_at = current_unix_timestamp();
```

3.  DBに新しい値を保存（上書き）。過去の履歴ログは一切保持しない。

### 3.2. 出題ウェイト計算関数： W(w)

単語 $w$ に対する出題の優先度（Weight）を算出する。この値が高い単語ほど、Weight Random Sampling において選ばれやすくなる。
※SQL上で計算するか、SQLiteから情報を一括取得してRust側で計算する。

$$ W(w) = (W_{diff} + W_{var} + W_{time}) \times \log_{10}(F_{corpus}) $$

*   $W_{diff}$ (Difficulty): `(2.0 - score_ema) * weight_mean`
*   $W_{var}$ (Instability): `variance_ema * weight_variance`
*   $W_{time}$ (Forgetting): `(now - last_reviewed_at) * time_decay_factor`
*   $F_{corpus}$ (Frequency): `words.sqlite3` の `words.frequency_count` (最低1.0でクリップ。未設定時は定数)

※`review_count == 0`（未学習）の場合の Learning Weight (すなわち $W_{diff} + W_{var} + W_{time}$) には一定の初期値（例: 変数全体の中央値、あるいは一律で高めの数値 `3.0` 等）を与える。

---

## 4. Sampling Strategy & Partitioning (Phase 2 & 3)

### Phase 2: 「新規(New)」と「復習(Review)」の枠分け抽出

フラッシュカードの1セッション（デッキ）を作成する際のロジック。
引数として `total_cards` (総出題数, 例: 10), `new_ratio` (新規の割合, 例: 0.2) を受け取る。

1.  **Review Cards 抽出**:
    *   **Target**: `review_count > 0` AND `is_ignored == false`
    *   **Count**: `total_cards * (1.0 - new_ratio)`
    *   **Method**: 上記 $W(w)$ に基づく **Weighted Random Sampling** (重み付きランダム抽出)。

2.  **New Cards 抽出**:
    *   **Target**: `review_count == 0` AND `is_ignored == false`
    *   **Count**: `total_cards * new_ratio`
    *   **Method**: 上記 $W(w)$ に基づく Weighted Random Sampling、または純粋に `words.frequency_rank` の上位からランダム・昇順抽出（コーパス頻度の高いものから順次消化させるため）。

### Phase 3: レベル（Tier）足切り

ユーザーの学習フェーズに合わせて、出題対象となる母集団に上限（Tier）を設ける。
引数として `active_tier_limit` (例: 3000) を設定可能にする。

*   **適用方法**: Phase 2のクエリ実行前に、フィルタ条件として `words.frequency_rank <= active_tier_limit` を追加するだけ。これにより処理が完全に直交する。

---

## 5. API / Interfaces (Tauri Commands)

Rust側のTauri Commandとして以下を実装し、Reactフロントエンドから呼び出せるようにする。

1.  `get_flashcard_deck(total_cards: u32, new_ratio: f32, tier_min: Option<u32>, tier_max: Option<u32>) -> Result<Vec<WordCard>, Error>`
    *   上記Phase 2 & 3のロジックを統合し、指定枚数のカード配列を返す。

2.  `submit_card_answer(word_id: i64, score: u8) -> Result<(), Error>`
    *   `score` (0, 1, 2) を受け取り、Phase 3.1 の EMA更新ロジックに基づき `user_learning_states` をUPSERTする。

3.  `set_word_ignored(word_id: i64, ignored: bool) -> Result<(), Error>`
    *   不要な既知単語（the, apple等）をサンプリング対象から外すフラグを切り替える。
