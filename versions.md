# versions

Runner Dock のツールチェインと主要依存の固定値である。`docs/04_TECHNOLOGY_STACK.md` §5 に従い、**確認値**（公式配布物が提供する版）と**導入値**（このリポジトリが実際にビルドへ使う版）を分けて書く。「最新」とだけ書かない。

依存の更新は独立した PR で行い、ロックファイルを手で編集しない。

## 1. 検証環境

| 項目 | 値 |
|---|---|
| 確認日 | 2026-09-20 |
| OS | Windows 11 Pro 10.0.26200 (x86_64) |
| host triple | `x86_64-pc-windows-msvc` |
| Linux 側 | WSL2 Ubuntu で `runnerdock-core` / `runnerdock-protocol` / `runnerdock-cli` / `runnerdock-guest` の単体試験を実行（`x86_64-unknown-linux-gnu`、同じ `rust-toolchain.toml`）。GitHub-hosted Linux での確認は CI (`.github/workflows/build.yml`) で行う |

WebView2 と MSVC ビルドツールは OS 側の前提であり、このファイルでは固定しない。導入手順は `docs/15_DEVELOPER_GUIDE.md` §2 にある。

## 2. ツールチェイン

| 対象 | 確認値 | 導入値 | 固定場所 | 確認元 | 備考 |
|---|---|---|---|---|---|
| Rust | stable 1.98.1 (48a229cea 2026-09-01) | 1.98.1 | `rust-toolchain.toml` | `rustup check` の stable 行 | edition 2024。`docs/04` §2 の候補と一致する |
| Node.js | 24.21.0 (Active LTS 系列) | 24.21.0 | `.node-version`、`package.json` の `engines` | 開発ホストの `node --version`、[Node.js Releases](https://nodejs.org/en/about/previous-releases) | Current 系列を追わない |
| pnpm | 12.5.1 | 12.5.1 | `package.json` の `packageManager` | 開発ホストの `pnpm --version` | lockfile と CI で同一版を使う |

## 3. Rust 依存

`Cargo.toml` の `[workspace.dependencies]` で `=` 付きの exact version として固定し、確定値は `Cargo.lock` が持つ。

| crate | 確認値 | 導入値 | 選定理由 |
|---|---|---|---|
| `tauri` | 2.11.6 | 2.11.6 | 2 系の最新安定版。3 系は alpha のため採用しない（ADR-011） |
| `tauri-build` | 2.6.3 | 2.6.3 | `tauri` 2.11 系に対応する build 時 crate |
| `serde` | 1.0.229 | 1.0.229 | DTO の直列化 |
| `serde_json` | 1.0.151 | 1.0.151 | IPC ペイロードと CLI の JSON 出力 |
| `thiserror` | 2.0.20 | 2.0.20 | 型付きエラー |
| `tracing` | 0.1.44 | 0.1.44 | 構造化ログ |
| `tracing-subscriber` | 0.3.23 | 0.3.23 | ログ出力先とフィルタ |
| `clap` | 4.6.7 | 4.6.7 | CLI の引数解析 |

確認元は crates.io の versions API（`https://crates.io/api/v1/crates/<crate>/versions`）で、yank 済みと prerelease を除いた最新安定版である。`tauri` と `tauri-build` は 2 系に限定して取った。

`tokio`、`reqwest`、`sqlx`、`windows`、`schemars` は LF-002 以降で導入する。使う段になってから確認値を取り直す。`anyhow` は使う crate ができた時点で追加する。

## 4. JavaScript 依存

`apps/desktop/package.json` で exact version として固定し、確定値は `pnpm-lock.yaml` が持つ。`.npmrc` の `save-exact=true` により caret 付きの追加を防ぐ。

| パッケージ | 確認値 | 導入値 | 選定理由 |
|---|---|---|---|
| `react` / `react-dom` | 19.3.0 | 19.3.0 | `docs/04` §2 の候補と一致する |
| `typescript` | 7.0.2 | **6.0.3** | 下の「確認値と導入値が異なる理由」を参照 |
| `vite` | 8.3.0 | 8.3.0 | `docs/04` §2 の候補と一致する |
| `@vitejs/plugin-react` | 6.1.1 | 6.1.1 | Vite 8 系に対応する版 |
| `vitest` | 5.0.1 | 5.0.1 | Vite 8 系を peer に含む |
| `@tauri-apps/api` | 2.11.1 | 2.11.1 | Rust 側 `tauri` 2.11 系と対応する |
| `@tauri-apps/cli` | 2.11.5 | 2.11.5 | 同上 |
| `eslint` | 10.11.0 | 10.11.0 | flat config |
| `typescript-eslint` | 8.70.0 | 8.70.0 | 型情報付き lint |
| `@testing-library/react` | 16.3.3 | 16.3.3 | コンポーネントの単体試験 |
| `jsdom` | 30.1.0 | 30.1.0 | Vitest の DOM 環境 |

確認元は npm registry の `latest` dist-tag（`npm view <package> version`）と、互換判定に使った `npm view <package> peerDependencies` である。

### 確認値と導入値が異なる理由

**TypeScript**: 公式の `latest` は 7.0.2 だが、`npm view typescript-eslint@8.70.0 peerDependencies` が返す peer 条件は `typescript >=4.8.4 <6.1.0` である。TypeScript 7 を入れると `pnpm lint` の型情報付き検査が成立しない。lint を落とすより型検査を保つ方を優先し、条件を満たす最新の 6.0.3 を導入値にした。`typescript-eslint` が 7 系へ対応した時点で、独立した更新 PR で再評価する。

**Rust**: `docs/04` §2 の候補は 1.98.1 で、開発ホストの `rustup` 既定は 1.98.0 だった。`rust-toolchain.toml` で 1.98.1 を固定し、その版でビルドと検査を実行した。

**Tauri**: crates.io の `tauri` の最新は 3.0.0-alpha.1 だが、prerelease を製品の前提にしない（ADR-011）。2 系の最新安定版 2.11.6 を導入値にした。

### cooldown の例外

`pnpm-workspace.yaml` の `minimumReleaseAgeExclude` は、`@tauri-apps/cli` 2.11.5 の解決時に pnpm が自動追加した公開直後 cooldown の例外である。版は上表の導入値と一致し、`--frozen-lockfile` を使う CI では解決自体が起きない。cooldown 方針を決めるときに見直す。

## 5. 固定の確認方法

```bash
rustc --version          # rust-toolchain.toml の channel と一致する
node --version           # .node-version と一致する
pnpm --version           # package.json の packageManager と一致する
cargo tree --workspace   # Cargo.lock の確定値を確認する
pnpm list --depth 0 -r   # pnpm-lock.yaml の確定値を確認する
```

固定値を変更する PR では、このファイルの確認日、確認値、導入値、選定理由を同じ PR で更新する。
