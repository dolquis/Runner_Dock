# 03. システムアーキテクチャ

**区分:** 採用設計 / **中心モデル:** Node → Backend → Runner

## 1. 構成と責務

```text
Desktop Renderer (React)
        │ Tauriの限定command / event
Desktop Rust Bridge
        │ Windows named pipe / 型付きフレーム
Windows User Agent                     GitHub API
  ├─ Reconciler / Operation manager ──────────┤
  ├─ Credential adapter / HTTP client         │
  ├─ SQLite / Audit / Metrics / Logs          │
  ├─ NativeWindowsBackend → 公式Runner ───────┤
  └─ WslBackend                              │
       └─ wsl.exeの継続セッション             │
            └─ Linux Guest → 公式Runner ─────┘
```

UIは表示と操作要求、Agentは状態と実行の管理、GuestはLinux内の限定操作を担当します。**公式Runnerがジョブを受けて実行する経路と、管理アプリがGitHub REST APIを見る経路は別**です。[S01](19_SOURCES.md#s01)[S02](19_SOURCES.md#s02)

## 2. Windows User Agent

初期実装はログオン中のWindowsユーザーで動く独立プロセスです。GUIの終了イベントで終了させません。WSLも同じWindows所有ユーザーの文脈で扱います。起動タスクをSYSTEMとして登録して私用WSLへ接続する設計にはしません。

Agentは同一SIDあたり1インスタンスに限定します。OSのロックとIPC待受を併用し、古いPIDファイルだけで存否を判断しません。GUIが起動した際には、既存Agentへhandshakeし、未起動の場合だけ製品同梱の検証済みAgentを開始します。

MVPはログオンセッション内運用です。画面ロックとログオフは区別し、ログオフ後・OS起動直後の無人継続は保証対象外です。

## 3. Backendの抽象化

BackendはRunnerのOSごとの実行場所を表します。`NativeWindows`と`Wsl`を先に実装します。WSLが複数ある場合はディストリビューションごとにBackend IDを割り当てます。

概念契約は以下です。これは型の方向性であり、現在存在するコンパイル済みAPIではありません。

```rust
// 概念例。最初のPoCでは列挙型によるdispatchを優先する。
enum BackendKind { NativeWindows, Wsl }

// 主要操作は detect / plan / provision / start / inspect / stop / remove。
// 戻り値は単なるboolではなく、Handle、観測値、Operation、構造化エラー。
```

Rustの`async fn`を含むtraitをそのまま`dyn Trait`化できるという前提は置きません。MVPでは`BackendKind`と静的dispatchで十分です。プラグイン化の必要が生じた場合に、boxed future等をADRで検討します。

## 4. WSL Guest

Guestは管理対象ディストリビューションに置く小さなRust実行ファイルです。Windows Agentが`wsl.exe --distribution ... --user ... --exec ...`で起動し、長寿命のstdio接続を維持します。GuestとRunnerは非rootで動かします。

Guestはヘルス照会、許可済みRunnerの起動・停止、ログ転送だけを受け付けます。一般的なリモートシェル、任意パスの削除、任意コマンド実行は提供しません。

systemdは検出情報として扱い、MVPの常駐必須条件にしません。systemdサービスだけではWSLの存続を保証しないため、Guestの接続・寿命を検証対象とします。[S04](19_SOURCES.md#s04)

## 5. 状態の管理

ボタンは直接`Start-Process`を呼びません。`StartNode`要求をAgentが受理し、desired stateを保存してOperationを作り、Reconcilerがobserved stateとの差分を埋めます。

永続状態に`Running`を書いて成功と見なさず、OSプロセス、Guest応答、GitHub観測を取り直します。処理は「少なくとも1回実行され得る」ことを前提に冪等化します。外部登録を伴う処理でexactly-onceを宣言しません。

GUIの再接続はsnapshotとevent sequenceで行います。イベントを欠落した場合はsnapshotを再取得し、古い差分を新しい状態へ適用しません。

## 6. 秘密情報の境界

通常画面にはcredential ID、権限要約、有効期限、接続状態だけを渡します。OAuth/Device Flowの秘密やPATを一般的な状態DTOに載せません。

手動PAT入力経路では入力画面からRustへの一時受渡しが必要です。「JavaScriptに一瞬も触れない」とは表現せず、ログ・永続store・DevTools・汎用イベントへ流さないことを要件にします。より厳密な入力隔離が必要ならネイティブ資格情報ダイアログを検討します。

Runnerへは管理用トークンを環境変数として継承しません。GuestにもGitHub管理トークンを保持させず、登録に必要な短命情報だけを限定転送します。

## 7. プロセス所有権

Windowsの子プロセスをJob Objectで追跡する設計を検証します。これはプロセス群の制御機構であり、同一ユーザーから秘密を守るサンドボックスではありません。[S25](19_SOURCES.md#s25)

Agentクラッシュ時に子プロセスを残すか終了するかは、LF-003の検証結果で確定します。初期推奨はAgent消失を検出して管理対象Runnerを停止するfail-stopです。継続実行を優先する方式には孤児プロセスの安全な再接続が必要です。

GUI終了ではAgentを止めないため、上記のfail-stopとは独立です。Guestは接続切断を検出し、復帰猶予の後にRunnerを停止する方針を持ちます。実行中ジョブが中断され得ることをUIと運用書に記載します。

## 8. 更新責任

本体、Agent、Guestは同じプロトコル互換表で管理します。RunnerはGitHub公式配布物と公式起動ラッパーを使い、自動更新を独自ループで破壊しません。Runnerの内部`.credentials`などをパースして管理DBへ複製しません。

ソフト更新とRunner自動更新は別々の責任です。更新のexit codeやラッパーの再起動挙動は実配布物で検証し、推測した`Runner.Listener`直起動に置き換えません。[S01](19_SOURCES.md#s01)[S21](19_SOURCES.md#s21)

## 9. 初期モノレポ構成

```text
apps/desktop/            Reactとsrc-tauri
crates/core/             ドメイン、ポリシー、Reconciler
crates/protocol/         IPC DTOと型生成
crates/agent/            Windowsホスト、各adapterの組立て
crates/guest/            WSL内限定実行プロセス
crates/backends/         windows / wsl adapter
crates/github/           REST、認証、rate limit
crates/storage/          SQLite、migration、credential参照
crates/cli/              操作CLI
packages/workflow/       YAML解析・変換ライブラリ
packages/contracts/      生成TypeScript型
fixtures/                API、Workflow、プロセスのテスト素材
docs/                    本文書群
```

Workflow解析は純粋関数中心のTypeScriptライブラリとしてUIとCLI用ツールから利用します。秘密情報もOS操作も持たせません。CLI用Nodeツールを製品ランタイムへ必須同梱するかは後続の判断で、MVPの生成画面はフロントへバンドルします。
