# 04. 技術選定とバージョン方針

**区分:** 採用提案 / **確認日:** 2026-09-19 / **注意:** 導入した exact version は `versions.md` が持つ

## 1. 最新技術を採用する基準

「新しい番号」より、現行安定版、型安全な境界、最小権限、公式に保守されるAPI、再現可能なビルドを優先します。最新技術を活用しても、nightly・RC・頻繁な無検証更新を製品の前提にしません。

## 2. 初期スタック

| 領域 | 基準・採用案 | 判断理由と固定方法 |
|---|---|---|
| Desktop | Tauri 2系 | capabilityを最小化し、Rust Agentと分離。exact patchはLF-001で固定 [S11](19_SOURCES.md#s11) |
| Core/Agent/Guest | Rust 1.98.1、edition 2024 | 公式告知で確認したstableを初期候補にする。`rust-toolchain.toml`に固定 [S15](19_SOURCES.md#s15) |
| UI | React 19.3 + TypeScript | React公式versionsで19.3を確認。TSのexact版は互換ビルド後に固定 [S16](19_SOURCES.md#s16) |
| Frontend build | Vite 8.3系 | 公式support一覧では8.3が通常修正対象。以前の8.1固定案を更新する [S17](19_SOURCES.md#s17) |
| 開発時JS runtime | Node.js 24 LTS | Currentを追うよりLTSを採用。公式release表で確認 [S18](19_SOURCES.md#s18) |
| Package manager | pnpmの検証済みstable | `packageManager`、lockfile、CIで同一版。版番号はLF-001で記録 |
| Async | Tokio | プロセス・I/O・タイマーを統合。ブロッキング処理は分離 [S31](19_SOURCES.md#s31) |
| GitHub HTTP | reqwest + serde | 使用endpointを薄いadapterに集約。未知フィールドに耐える |
| 状態保存 | SQLite + SQLx | migration、トランザクション、型付きアクセス。UIにSQL権限を渡さない [S32](19_SOURCES.md#s32) |
| Query cache | TanStack Query v5系 | Agentのsnapshot取得・無効化・再接続を扱う [S33](19_SOURCES.md#s33) |
| 秘密保存 | Windows Credential Manager adapter | UI・DBへ秘密本体を保存しない。Agent側でのみ取扱う [S27](19_SOURCES.md#s27) |
| Windows API | windows-rs | SID、Job Object、pipe ACL等の呼出しを小さく包む |
| 診断 | tracing + JSON logs | correlation ID、レベル、ローテーション、機密の除外 |
| UI test | Vitest / Testing Library | コンポーネント、状態、アクセシビリティ |
| 実アプリE2E | WebdriverIO + Tauri対応service | 現行のTauri公式テスト案内に沿い、テスト専用機能を配布物へ入れない [S14](19_SOURCES.md#s14) |
| 本体更新 | Tauri Updater | 配布署名とUpdater署名を区別して運用 [S13](19_SOURCES.md#s13) |

Rust/React/Viteの値は確認時点の資料に基づく開始候補です。利用開始日が後なら改めて確認してください。`latest`という浮動指定をリリースの再現手順に残しません。

## 3. 依存関係を最初に増やしすぎない

UIはCSS variablesによるdesign tokenを基本とし、コンポーネントライブラリは必要なプリミティブだけを採用します。巨大なグラフライブラリ、ターミナルエミュレーター、Webサーバーを最初から常駐させません。

Redux、Zustand等の追加storeは、TanStack Queryと小さいUI状態だけで不足した場合に導入します。同一のRunner状態をAgent、Query cache、global storeでそれぞれ正規化し直しません。

React Compilerは測定用ブランチで評価可能としますが、MVPの必須要件にしません。ローカルデスクトップに不要なNext.js、SSR、React Server Componentsも採用しません。

## 4. 型の共有

`crates/protocol`をワイヤー契約の正とし、serde DTOからJSON Schema/TypeScriptを生成する薄いツールを作ります。候補はschemarsと検証済み型生成ツールですが、crateのstable状況・nullable/enum/u64表現をLF-002で確認して固定します。

RCの便利なbindingを理由に通信契約をツール依存へ固定しません。GitHub数値IDは内部で整数、UIとの境界では10進文字列に変換し、JavaScriptの整数精度差を回避します。

## 5. バージョン固定の成果物

次のファイルで版を固定します。確認値と導入値の対応は[versions](../versions.md)にあります。

```text
rust-toolchain.toml     Rust exact version / components / profile
Cargo.lock              Rust依存の確定値
package.json            packageManager exact version
pnpm-lock.yaml          JS依存の確定値
.node-version           開発時Nodeのexact version
versions.md             確認日、公式URL、検証したOS、選定理由
```

`versions.md`に「最新」とだけ書かず、確認値と導入値を分けます。依存更新は独立PRにし、ロックファイルを手で編集しません。

## 6. 候補比較

| 選択肢 | 採否 | 理由 |
|---|---|---|
| Tauri 2 | 採用 | UI技術とRust制御層を使い分ける。安全性は実装次第であり自動保証ではない |
| Electron | 不採用 | 初期の要件にはTauriで足りる。将来のAPI不足で再検討可能 |
| PowerShellのみ | 検証用 | OS挙動の再現には使うが、秘密管理・状態機械・更新の主実装にはしない |
| 常時管理者Windowsサービス | 初期不採用 | 個人WSL・権限・ログオン文脈を複雑にする |
| GitHub App + Device Flow | 主経路候補 | 公開デスクトップで共有secretを配らず認証する検証を先行 [S08](19_SOURCES.md#s08)[S09](19_SOURCES.md#s09) |
| 認可コード + PKCEだけ | 単独案を棄却 | GitHub Appのcode交換はclient_secret要件も持つ [S08](19_SOURCES.md#s08) |
| 独自Runnerプロトコル | 不採用 | GitHub公式Runnerを再利用し、管理機能へ集中する |
| Kubernetes/ARC | 非目標 | 自宅Windows PCの2環境管理に必要ではない |

## 7. 将来技術の入口

Docker、使い捨てVM、GitHub Actions Runner Scale Set ClientはBackendや割当制御の後続候補です。公式Scale Set ClientはGo製の部品であり、Rustライブラリとして直接使えるとは扱いません。採用するなら別プロセス統合と保守範囲を評価します。[S01](19_SOURCES.md#s01)
