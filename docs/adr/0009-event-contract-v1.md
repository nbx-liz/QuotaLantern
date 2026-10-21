# ADR-0009：内部イベント契約 v1

- ステータス：Proposed
- 決定者：技術責任者＋セキュリティ担当
- 期限：`crates/protocol` のマージ前
- 追跡：#3

## 背景

取得部からコアへ渡すデータは、提供元の生の JSON ではなく、QuotaLantern 独自の正規化イベントに限る（ADR-0001）。
引き継ぎ時のスキーマ案 v0.1 を検証したところ、次の問題があった。

1. 日時を JSON Schema の `format: date-time` だけで検証していた。
   Draft 2020-12 では `format` は既定で注釈にすぎず、検証器の設定や追加ライブラリの有無で黙って無効になる。
   実際に、案に付属した Python の依存指定では `"received_at": "yesterday"` がスキーマを通った。
2. 仕様は日時を UTC としているが、`date-time` は `+09:00` などの時差付きも通す。
3. UUID も `format: uuid` だけで検証していた。大文字、括弧付き、ハイフンなし等の表記の扱いが実装によって異なる。
4. `source_state` が指標の種類（`target`）しか持たず、複数の枠のうちどれが stale かを表せない。
5. 重複キーは JSON Schema では表現できず、パーサー任せだった。

## 決定（案）

### 正本と検証の二層

- 手書きの `schemas/event.schema.json` を契約の正本とする。
- Rust の `crates/protocol` は、スキーマと同じ合否を返す厳密なパーサーとして実装する。
- `tests/fixtures/events/manifest.json` の全 fixture を、スキーマ検証器と Rust パーサーの両方で判定し、
  期待どおりであることを CI で強制する。
- スキーマでは表現できず、パーサーだけが拒否する入力は、manifest に `reject_layer: "parser"` と明記した種類に限る。

| 拒否する層 | 対象 |
| --- | --- |
| スキーマとパーサーの両方 | 未知キー、型・値域・列挙の違反、日時・UUID の書式違反、`schema_version` の不一致、配列（契約に配列型の項目はない） |
| パーサーだけ | 重複キー、16 KiB 超、入れ子の深さ 16 超、整数項目の小数・指数・負のゼロ表記（`1.0`、`1e3`、`-0`。`schema_version` を含む）、不正な UTF-8、JSON として不正な入力（`NaN` を含む） |

serde の derive した構造体は、オブジェクトの代わりに位置指定の配列も受け付けてしまう。
契約には配列型の項目がないため、パーサーは解釈の前に配列を一律に拒否し、スキーマと同じ判定にする。

### 書式の変更（v0.1 案からの差分）

- **日時**：`received_at`、`source_at`、`resets_at` は UTC の `Z` 表記だけを受け付ける。
  `^\d{4}-\d{2}-\d{2}T([01]\d|2[0-3]):[0-5]\d:[0-5]\d(\.\d{1,9})?Z$` の `pattern` を `format` と併記し、
  書式の検証が `format` の設定に依存しないようにする。暦として存在しない日付はパーサーが拒否し、スキーマ側は `format` で拒否する。
  うるう秒（`:60`）は受け付けない。
- **UUID**：`run_id`、`source_id`、`entity_id`、`bucket_id`、`counter_epoch`、`subject_id` は、
  小文字・ハイフン区切りの UUID version 4（RFC 4122 variant）だけを受け付ける。
  `^[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$`
  いずれも起動ごと・接続ごとのランダムな一時 ID であり、永続 ID や実アカウントの ID を入れない。
- **source_state**：`subject_id`（UUID または `null`）を必須で追加する。
  `target` が `quota` なら枠の `bucket_id`、`context` / `token_counter` なら `entity_id` を指す。
  `null` はその取得元の当該指標すべてを指す。`target` が `source` のときは `null` でなければならない。

### パーサーの規則

- サイズ（16 KiB）と入れ子の深さ（16）を、JSON として解釈する前に検査する。
- すべての構造体で未知のフィールドを拒否する。
- 失敗は固定のエラーコードだけで返す。入力の断片やライブラリのエラーメッセージを外へ出さない。
  `schema_version` が 1 以外の整数リテラルの場合は、接続を止める判断に使えるよう他の失敗と区別する。

### 依存の追加

| crate | 用途 | 種別 |
| --- | --- | --- |
| serde（derive） | 型付きの解釈 | 実行時 |
| serde_json（float_roundtrip） | JSON の解釈。小数を正しく丸めて読み、`used_percent` の境界値でスキーマと判定を揃える | 実行時 |
| time（parsing） | RFC 3339 の暦の検証 | 実行時 |
| uuid | UUID の型 | 実行時 |
| jsonschema（既定機能なし） | スキーマとパーサーの一致を確かめる試験 | 開発時のみ |

`jsonschema` は既定機能を無効にし、HTTP・ファイルからのスキーマ取得を含めない。

`jsonschema` は serde_json の `float_roundtrip` を有効にするため、開発時の依存だけに頼ると、
試験のビルドと本番のビルドで小数の読み方が変わる（Cargo の feature の統合による）。
そのため `float_roundtrip` は実行時の依存として明示し、CI で本番の依存グラフに含まれることを確かめる。
いずれも MIT / Apache-2.0 互換のライセンスであることを確認する。

## 検討した代替案

- Rust の型を正本にして `schemars` でスキーマを生成する：UUID や日時の厳密な書式を生成器に任せることになり、
  生成結果の差分を追う手間も残る。プロダクト責任者の判断で手書きスキーマを正本とした。
- `format` だけで日時と UUID を検証する：v0.1 案で起きた「黙って無効になる」問題が残る。

## 影響

v0.1 案のスキーマと fixture とは互換性がない（`source_state.subject_id` が必須、日時と UUID の書式が狭い）。
v0.1 案は公開しておらず、利用者もいないため、`schema_version` は 1 のままとする。

## 受入条件

- 全 fixture について、スキーマの判定と Rust パーサーの判定が manifest の期待どおりになる。
- スキーマ検証器の `format` が実際に有効であることを、試験自身が確かめる。
- `+09:00` の日時、大文字の UUID、`target: source` に `subject_id` を持つ `source_state`、
  各 metric 種別での重複キー、整数の小数・負のゼロ表記、配列形式、16 KiB 超、深さ 17 の入力を、それぞれ拒否する試験がある。
- 本番の依存グラフで serde_json の `float_roundtrip` が有効である。
- エラーコードに入力の内容が含まれない。
