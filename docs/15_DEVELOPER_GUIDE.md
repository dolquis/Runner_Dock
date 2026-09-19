# 15. 開発者ガイド

**対象:** 人間の開発者・AIコーディングエージェント / **状態:** 初期リポジトリ作成前

## 1. 最初のリポジトリ

この文書一式を新規repositoryへ配置し、[START_DEVELOPMENT](../START_DEVELOPMENT.md)と[AGENTS](../AGENTS.md)を入口にします。既存HomeRunのgit履歴、ソース、アセット、設定をコピーしません。製品名・crate prefixは仮称のため、表示名を変更しやすくします。

初期構成は次を提案します。空のcrateを大量に作る必要はなく、LF-001ではcore/protocol/agent/guest/desktopの最小構成を優先します。

```text
apps/desktop/             React UI / src-tauri bridge
crates/core/              pure domain / reducer / policy
crates/protocol/          DTO / schema / versioning
crates/agent/             user-session supervisor
crates/guest/             WSL-side supervisor
crates/backends/          native Windows / WSL adapters
crates/github/            API / auth / rate limit
crates/storage/           SQLite / credential references
crates/cli/               typed IPC client
packages/contracts/      generated TypeScript
packages/workflow/       pure parser / selector generator
fixtures/                fake API / state / YAML / logs
scripts/                 reviewed build and test utilities
docs/                    this documentation
```

CoreはOS、Tauri、HTTPに依存させません。protocolに秘密そのものを通常DTOとして追加しません。Workflowの純粋ライブラリはブラウザ上で動かせる範囲にし、任意shell実行を持ち込まない設計にします。

## 2. Windows開発環境

TauriのWindows前提となるC++開発ツールとWebView2、選定Rust、Node.js 24 LTS、固定pnpmを準備します。Visual Studioの具体的なworkload/SDKはTauri公式の現行案内とLF-001の実ビルドで確定します。[S12](19_SOURCES.md#s12)[S18](19_SOURCES.md#s18)

最初に次の読取コマンドで実際の環境を記録します。

```powershell
rustc --version
cargo --version
node --version
pnpm --version
wsl.exe --version
wsl.exe --list --verbose
```

未導入ならコマンド失敗を診断結果として残します。ビルドできる前にWSLの再インストールや`--unregister`を実行しません。管理者権限が必要な前提導入と、通常のアプリ開発・起動を分離します。

## 3. Mock-first開発

初期UIとAgentのテストは、ネットワーク・PAT・GitHub App・Runnerを不要にします。Mock fixtureにはIdle、Busy、Offline、Unknown、Stale、Creating、PartialFailure、RateLimited、ReauthenticationRequiredを含めます。

実GitHubへの接続は設定と起動引数で明示的に選択し、Mockから自動的に本番接続へ落ちる実装を禁止します。実Runnerを扱うE2Eは専用private repositoryと専用の作業領域で行います。

## 4. 初期化後に定義する開発コマンド

以下は**LF-001で作成するコマンド契約**です。現時点の文書パッケージで実行可能なアプリはありません。package scriptの実体を作成した後にREADMEへ実測済み手順として移します。

| コマンド案 | 実装する動作 |
|---|---|
| `pnpm dev:mock` | 実GitHub/WSLへ接続せずUIを表示 |
| `pnpm desktop:dev` | Tauri UIを起動しuser Agentへ接続 |
| `pnpm lint` | TypeScript、React、禁止APIの検査 |
| `pnpm test` | UI/Workflowの単体・fixture試験 |
| `pnpm contracts:check` | Rust由来の契約生成物との差分検査 |
| `pnpm e2e:mock` | テスト専用Tauri buildのUI E2E |

Cargo側はworkspace完成後、`cargo fmt --all -- --check`、`cargo clippy --workspace --all-targets -- -D warnings`、`cargo test --workspace --locked`を基準にします。Windows専用crateはplatform cfgで囲み、Linux CIからWindows実装を検証したと誤認しないようjobを分けます。

## 5. コーディング規約

Rustは機能失敗を型付きErrorへ変換し、ユーザー操作・外部I/Oで`unwrap()`しません。blocking APIをasync executorで待ち続けないよう分離し、timeout/cancellationを付けます。再試行は冪等な読取と外部副作用のある書込で別方針にします。

ReactはAgent snapshotを正とし、イベントでQuery cacheを更新・無効化します。APIやIPCの長い処理をUI threadで同期実行せず、mountごとに増殖するevent listenerを解除します。ログ画面は上限付き・必要なら仮想化し、計測値の更新で画面全体を再描画しないようにします。

`any`、未知JSONの無検証cast、文字列結合shell、汎用`exec` IPCを避けます。秘密を`Debug`、例外、telemetry、clipboardへ混入させないレビューを行います。

## 6. CIの基本構成

| 実行環境 | 主な確認 |
|---|---|
| GitHub-hosted Linux | core/protocol、Workflow generator、TS、文書リンク、schema |
| GitHub-hosted Windows | Windows crate、Tauri build、IPC、Mock UI、配布artifact検査 |
| 明示許可した実機Windows/WSL | lifecycle、Agent/Guest切断、公式Runner更新、WSL idle、認証E2E |

このプロジェクト自身の外部PRを、開発者の私用self-hosted PCで無条件に実行しません。PRから署名秘密や管理PATに届かない構成にします。[S19](19_SOURCES.md#s19)

Actionsのバージョンはリリース時に検証した完全commit SHAへ固定し、更新PRで説明します。Windows hostedの使用量を抑えるため純粋ロジックはLinuxで検査しますが、必要なWindows試験を省略して「節約成功」としません。

## 7. 並行開発の境界

core/protocol、Windows、WSL、GitHub、UI、Workflowの担当を分けられます。共有schema、DB migration、lockfileの変更は調整担当を明示します。別worktreeでも外部GitHub環境や同じdistroを共有すれば衝突するため、integration test対象も分離します。

PRには目的、対象LF/FR/TC、変更契約、実施コマンド、結果、未実施試験、外部変更、rollbackを含めます。差分の大きさではなく、検証できる単位を優先します。

## 8. 最初の開発指示例

> AGENTS.mdとSTART_DEVELOPMENT.mdを読み、LF-001とLF-002だけを実装してください。まず計画と対象ファイルを示し、core/protocol/Mockの契約を固定してください。実GitHub API呼出し、Runner登録、WSL設定変更、Secrets作成、署名鍵生成は行わないでください。終了時に実行したテストと未実施テスト、仕様からの差分を報告してください。

この指示は実装開始用です。制約を解除する場合は作業単位ごとに対象と副作用を確認します。
