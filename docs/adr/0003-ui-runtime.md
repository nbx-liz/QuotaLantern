# ADR-0003：UI ランタイム（Tauri 2 の条件付き採用）

- ステータス：Proposed
- 決定者：技術責任者＋セキュリティ担当
- 期限：G0 終了
- 追跡：#1

## 背景

Tauri 2 には常駐アイコンと、フロントエンドの権限を制御する仕組み（capabilities）がある。
一方で、Windows では WebView2、macOS では WKWebView を使う。
WebView2 のユーザーデータフォルダーには保存されるものがあり、WebKit には非永続のデータストアがある。
いずれも、Tauri で使ったときにすべての保存経路が安全であることを自動では保証しない。

出典（いずれも 2026-10-21 確認）：

- <https://v2.tauri.app/learn/system-tray/>
- <https://v2.tauri.app/security/capabilities/>
- <https://v2.tauri.app/reference/webview-versions/>
- <https://learn.microsoft.com/en-us/microsoft-edge/webview2/concepts/user-data-folder>
- <https://developer.apple.com/documentation/webkit/wkwebsitedatastore/nonpersistent()>

## 決定（案）

Tauri 2 を、次のゲートに両 OS で合格することを条件に採用する。

| ゲート | 合格条件 |
| --- | --- |
| UI-G1 | 使用量や識別子が、WebView のストレージ、キャッシュ、復元状態に残らない |
| UI-G2 | 外部ページ・外部フォント・広告・解析 SDK を読み込まず、外部へのナビゲーションを拒否する |
| UI-G3 | 任意の shell / filesystem / HTTP / DB / store の機能を UI に公開しない |
| UI-G4 | リリースビルドの console、panic、クラッシュ・復元機能を含めて監査する |
| UI-G5 | 自動テストと異常終了の試験で、回帰を検出できる |

- 自作の Tauri コマンドも明示的な許可の対象にする。登録したコマンドが既定ですべてのウィンドウから呼べる挙動に頼らない。
- CSP は同梱の資産だけを許可し、必要な内部 IPC 以外の接続を拒否する。厳密な設定は採用版で検証する。
- 終了時のキャッシュ削除だけで合格にしない。起動中の書き込みと異常終了を含め、書き込みの時点と保存内容の両方で判定する。

## 検討した代替案

- 不合格の場合：WebView を使わないネイティブの表示層に切り替える。共通コアとイベント契約は維持し、新しい UI 方式も同じ保存・通信の監査を通す。フレームワークの選択より非保存要件を優先する。

## 影響

`apps/desktop/` は、この ADR が Accepted になるまで workspace に含めない。

## 受入条件

UI-G1〜G5 を macOS と Windows の両方で判定し、試験環境・対象版・観測範囲・未検証の領域を記録する。
