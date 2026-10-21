# ADR 台帳

QuotaLantern の設計上の決定を、1 件 1 ファイルで記録する。
Accepted の ADR は、このリポジトリで最も優先度の高い文書である（[CONTRIBUTING.md](../../CONTRIBUTING.md)）。

## ステータス

| ステータス | 意味 |
| --- | --- |
| Proposed | 案。実装の根拠にしてよいが、決定ではない。受入条件を満たすまで公開しない |
| Accepted | 決定済み。変更するには新しい ADR で置き換える |
| Rejected | 検討の結果、採用しない |
| Superseded | 後続の ADR に置き換えられた |

## 役割

| 役割 | 決めること | 担当 |
| --- | --- | --- |
| プロダクト責任者 | 範囲、表現、例外の承認 | @nbx-liz |
| 技術責任者 | 取得方式、データ契約、OS 対応 | @nbx-liz |
| セキュリティ担当 | 非保存、権限、通信のレビュー | @nbx-liz |

1 人で開発している間も、レビューの記録は役割ごとに分けて残す。
安全性は自己申告だけで合格にせず、試験の証跡を ADR か Issue に添付する。

## 一覧

| ADR | 論点 | ステータス | 決定者 | 期限 |
| --- | --- | --- | --- | --- |
| [0001](0001-core-architecture.md) | 共通 Rust コアと OS 別 UI | Proposed | 技術 | G0 終了 |
| [0002](0002-unavoidable-metadata.md) | 避けられない識別メタデータの受信 | Proposed | プロダクト＋セキュリティ | 実機での取得試験の前 |
| [0003](0003-ui-runtime.md) | UI ランタイム（Tauri 2 の条件付き採用） | Proposed | 技術＋セキュリティ | G0 終了 |
| [0004](0004-official-client-side-effects.md) | 公式クライアントを動かすことの副作用 | Proposed | 技術＋セキュリティ | G0 終了 |
| [0005](0005-setup-configuration.md) | 導入設定の保存と解除 | Proposed | プロダクト | G1 開始 |
| [0006](0006-platform-targets.md) | 対応 OS 版と CPU | Proposed | 技術 | G1 開始 |
| [0007](0007-license-and-publication.md) | ライセンスと公開 | Accepted | プロダクト | — |
| [0008](0008-name-and-distribution.md) | 名称、公開主体、配布費用 | Proposed | プロダクト | 公開前 |

ADR-0009（内部イベント契約 v1）は `crates/protocol` の実装前に起票する。

## 未決論点（ADR 未起票）

| ID | 論点 | 関係 | 決める時期 |
| --- | --- | --- | --- |
| Q-01 | stale の判定条件。暫定案は「受信間隔 3 分超」だが、Claude Code の Status line は応答のたびにしか更新されず、作業していない間はほぼ常に stale になる。「更新がない」と「提供元の障害」を区別する表示方針が要る | FR-03、ADR-0002 | Claude 取得試作（QL-002）の後 |
| Q-02 | 既存の Status line との共存。設定できるコマンドは 1 つで、既存の設定を上書きしない方針と両立する導入手順が要る | ADR-0005、FR-01 | QL-002 の後 |
| Q-03 | Codex の能動照会の間隔。暫定案は 60 秒、同時 1 件、タイムアウト 10 秒、失敗時は最大 15 分までの指数バックオフ。公式の推奨値ではない | ADR-0004 | Codex 取得試作（QL-003）の後 |
| Q-04 | 1 つの Status line 取得プロセスから、同一ユーザーの別プロセスによる偽装接続をどこまで排除できるか | ADR-0002、IPC | IPC 設計時 |

## 書式

新しい ADR は [`template.md`](template.md) をもとに作る。番号は連番で、再利用しない。
