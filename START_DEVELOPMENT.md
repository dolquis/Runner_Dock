# 開発開始指示書

**状態:** 開始可能な作業指示 / **初期到達点:** 本番Runnerを壊さない検証基盤と縦断PoC

## 最初の目標

初回から完成アプリを一括生成しません。まず「1つのWindows AgentがWindowsとWSLのダミー実行プロセスを同時管理し、GUIを閉じてもAgentが残り、再接続すると状態が復元される」までを作ります。GitHubへの実Runner登録は、その後の明示承認付きの検証です。

## リポジトリを作ったら

このパッケージの`README.md`、`AGENTS.md`、`docs/`、`templates/`を初回コミットに入れます。製品名はRunner Dock、内部識別子は`runnerdock`です。商標・既存名との重複確認は公開前の確認事項として残ります。文書のみを置いた状態でリリースタグを作成しません。

ブランチ例は`feat/LF-001-workspace`。GitHubリポジトリの新規作成や公開範囲の変更は、所有者が指定した場合にだけ行います。

## 最初の3つのPR

| PR | 対象 | 必須成果 | 明示的にしないこと |
|---|---|---|---|
| A | LF-001 / LF-002 | Cargo/pnpm workspace、型付き契約、MockBackend、CI、固定バージョン記録 | 実Runner登録、既存WSLの変更 |
| B | LF-003 / LF-004 / LF-005 | Windows/WSLダミー子プロセス、寿命・停止・二重起動・IPCの検証結果 | PAT保存、任意コマンドのGUI公開 |
| C | LF-006 / LF-010の薄い縦断 | GitHub App Device Flow検証、Node画面、操作進捗、Mockによる再接続 | 未承認repoへのRunner作成、Workflow自動適用 |

Aのプロトコル合意後、Windows/WSL検証とMock UIは並行化できます。認証と安全なプロセス管理の検証を、見た目の完成を理由に後回しにしません。

## エージェントへ渡す開始プロンプト

```text
このリポジトリのAGENTS.mdとSTART_DEVELOPMENT.mdを読み、LF-001とLF-002を実装してください。
HomeRunのフォーク・ソース移植は行わず、既存の開発文書を設計基準にします。

今回の成果は次に限定します。
- Tauri 2 + Rust + React/TypeScriptのworkspace骨格
- Node/Backend/Runnerと希望状態・観測状態の型
- バージョン付きIPC契約とMockBackend
- MockでStart/Stop/Unknown/Busyを示す最小画面
- 単体・型・lint・buildのCIと依存バージョン固定
- 実行した検証と未実行項目の記録

実機のサービス、WSL設定、GitHub App、Runner、Secrets、Workflowは変更しないでください。
文書と実装に不一致が出たら黙って仕様を変更せず、ADR変更案を記録してください。
必要な共有契約を先に確定し、並行作業では担当ファイルを分離してください。
最後に変更ファイル、テスト結果、未確認事項、次に着手できるLFタスクを報告してください。
```

## 技術検証の合格ゲート

**G1 プロセス:** Windows Runnerの公式起動ラッパーと更新挙動を崩さず管理できる。ダミー→テストRunnerの順に確認する。

**G2 WSL:** GUIなし、アイドル状態、長時間起動、再接続を経てもGuestが維持され、対象外のWSL作業に影響しない。systemdだけによる常駐を前提にしない。[S04](docs/19_SOURCES.md#s04)

**G3 認証:** GitHub App Device Flowの初回認証とrefreshが共有`client_secret`を配布せず完結することを確認する。公式資料ではdevice flow由来のrefreshに例外規定がある。[S08](docs/19_SOURCES.md#s08)[S09](docs/19_SOURCES.md#s09)

**G4 安全境界:** 非昇格運用、ローカルIPCの権限制御、機密情報の非継承、破壊操作の拒否を確認する。

**G5 実CI:** 所有者が指定したテスト用private repoでWindows/Linuxの両ジョブを実行し、停止時にはhostedへ事前選択されることを確認する。

## まだ決めなくてよいこと

公開名称、商標、公式ロゴ、有料プラン、macOS対応、Docker・Scale Sets、任意Workflowの自動変換は初期PoCの阻害条件にしません。公開ライセンスはMIT OR Apache-2.0に決定済みです。配布署名方式は外部公開前の承認事項です。
