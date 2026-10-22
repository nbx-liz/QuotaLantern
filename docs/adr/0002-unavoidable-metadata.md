# ADR-0002：避けられない識別メタデータの受信

- ステータス：Accepted（2026-10-22、実機での取得試験の前に承認）
- 決定者：プロダクト責任者＋セキュリティ担当
- 追跡：#1、#5

## 背景

Claude Code の Status line は、標準入力の JSON に利用枠（`rate_limits.*.used_percentage`、`resets_at`）と
コンテキスト（`context_window.used_percentage`）を含む。
同じ入力には、セッション ID、作業パス、transcript_path などの識別メタデータも含まれる。

出典：<https://code.claude.com/docs/en/statusline>
（2026-10-22 に Claude Code 2.1.306 の時点で再確認。）

再確認した事実：

- `rate_limits` は Pro / Max の契約者（または Claude apps gateway 経由）にだけ、セッションの最初の応答の後に現れる。
  `five_hour` と `seven_day` はそれぞれ独立に欠けることがあり、`resets_at` を過ぎた枠は Claude Code が入力から外す。
- `used_percentage` は 0〜100、`resets_at` は Unix エポック秒。
- `rate_limits.spend_limit` は gateway 経由のときだけ現れ、`used_percentage` が 100 を超えることがあり、米ドルの金額も含む。
- `context_window.used_percentage` はセッションの初期に `null` になり得る。
- Status line は、イベント発生時（300ms のデバウンス）、枠の `resets_at` 到達時、任意の `refreshInterval` ごとに実行される。

「本文と認証情報を取得しない」ことと、「識別メタデータを一度も受信しない」ことは別の要件である。
後者まで必須とするなら、Status line の方式は採用できない。

## 決定

受信した後で破棄する方式を採る。

### 許可する項目と変換

| 入力 | 変換先 | 扱い |
| --- | --- | --- |
| `rate_limits.five_hour.used_percentage` / `.resets_at` | 5 時間枠の使用率と、UTC のリセット時刻 | 0〜100 の範囲外、またはエポック秒として不正なら、その枠を固定のエラーにする。丸めない |
| `rate_limits.seven_day.used_percentage` / `.resets_at` | 7 日枠の使用率と、UTC のリセット時刻 | 同上 |
| `context_window.used_percentage` | コンテキスト使用率 | `null` や欠落は「未取得」。0 にしない |
| `context_window.context_window_size` | コンテキストの容量（トークン） | 任意 |
| `rate_limits.spend_limit` | なし | 存在だけを「非対応」として記録する。100% を超え得ること、金額を含むことから、契約枠として扱わない |

上記以外の項目は、解釈の途中で読み飛ばし、値を複製しない。

### 取得部の規則

- 取得部は型付きの小さなパーサーで、許可した項目だけを取り出す。
- 提供元の入力は 1 MiB と入れ子の深さ 16 を、解釈の前に検査する。内部イベント契約と違い、配列は許す。
- Status line のコマンドは更新のたびに新しいプロセスとして起動されるため、接続ごとの識別子や連番を持てない。
  取得部は中間の型付きの値を返すだけにし、内部イベントの封筒（`run_id`、`source_id`、`seq`、`received_at`）は、
  常駐する IPC の受け手が付ける（IPC の設計で決める）。
- コマンドは常に終了コード 0 で終わり、標準出力には短い表示か固定の文字列だけを出す。
- 作業パス、会話名、transcript_path、リポジトリ名はコアにも UI にも渡さない。入力全体を保存しない。
- 未知のフィールドは構文として読み飛ばし、その値を診断に出さない。
- 標準エラーに生の入力を出さない。接続に失敗したら速やかに終了する。IPC への送信を待つ上限は暫定で 100ms。
- 追跡するコンテキストは、選んだ 1 セッションを基本とする。
  セッションの対応付けが必要なら取得部のメモリ内に限り、UI には「セッション 1」のような一時ラベルだけを渡す。
  生の ID をハッシュ化して永続化することも禁止する。
- `total_input_tokens` 等は直近の応答に基づくコンテキストの値であり、累積消費として加算しない。

## 検討した代替案

- OpenTelemetry：トークンの計測経路はあるが、メールアドレスや組織 ID などの属性も出力される。
  設定だけで除去できるとは断定できない。初期版では無効とする。
  後で採用する場合も metrics に限り、logs / traces / prompt / response のイベントは有効にしない。
  出典：<https://code.claude.com/docs/en/monitoring-usage>（2026-10-21 確認）
- 識別メタデータを一切受信しない方式：Status line 以外の公式な経路が見つかっていない。

## 影響

不要な識別メタデータが、短時間ながら取得部のメモリを通過する。この点を利用者向けの説明に明記する。

## 受入条件

- 合成のメールアドレス・パス・秘密値を混ぜた入力で、それらがコア・UI・ログ・永続領域に到達しない。
- Status line の終了コードと stdout の扱いが、Claude Code の動作を妨げない（実機で確認）。
- 不正な JSON、過大な入力、連続した入力で、固定のエラーと上限が働き、ホストを止めない。

## 記録

- 2026-10-22：プロダクト責任者・セキュリティ担当（@nbx-liz）が、実機での取得試験の前に承認した。
  許可する項目と変換は、Claude Code 2.1.306 の公式文書を再確認して確定した（#5）。
- 2026-10-22：Linux 上の実機で、入力のキーと型だけを記録した（実機試験 A、値は記録していない）。
  - 許可した 4 項目（`five_hour`、`seven_day`、`context_window.used_percentage`、
    `context_window.context_window_size`）は、すべて数値として届き、取り出せた。`spend_limit` は無かった。
  - 識別につながるキーとして `session_id`、`session_name`、`prompt_id`、`transcript_path`、`cwd`、
    `workspace.current_dir`、`workspace.project_dir`、`workspace.added_dirs`、`scratchpad_dir` が届いた。
    取得部はどれも読み取らない。
  - 公式文書の一覧に無いキー（`prompt_cache`、`effort`、`thinking` など）も届いた。
    未知のフィールドとして読み飛ばされ、取り出しに影響しなかった。
  - 小文字の識別子でないキーを `<dynamic>` に伏せる処理は、今回の入力では一度も働かなかった。
