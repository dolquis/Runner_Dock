# versions

Runner Dock のツールチェインと主要依存の固定値である。`docs/04_TECHNOLOGY_STACK.md` §5 に従い、**確認値**（公式配布物が提供する版）と**導入値**（このリポジトリが実際にビルドへ使う版）を分けて書く。「最新」とだけ書かない。

依存の更新は独立した PR で行い、ロックファイルを手で編集しない。

## 1. 検証環境

| 項目 | 値 |
|---|---|
| 確認日 | 2026-09-20 |
| OS | Windows 11 Pro 10.0.26200 (x86_64) |
| host triple | `x86_64-pc-windows-msvc` |
| Linux 側 | このリポジトリでは確認していない。GitHub-hosted Linux での確認は CI (`.github/workflows/build.yml`) で行う |

WebView2 と MSVC ビルドツールは OS 側の前提であり、このファイルでは固定しない。導入手順は `docs/15_DEVELOPER_GUIDE.md` §2 にある。

## 2. ツールチェイン

| 対象 | 確認値 | 導入値 | 固定場所 | 備考 |
|---|---|---|---|---|
| Rust | stable 1.98.1 (48a229cea 2026-09-01) | 1.98.1 | `rust-toolchain.toml` | edition 2024。`docs/04` §2 の候補と一致する |
| Node.js | 24.21.0 (Active LTS 系列) | 24.21.0 | `.node-version`、`package.json` の `engines` | Current 系列を追わない |
| pnpm | 12.5.1 | 12.5.1 | `package.json` の `packageManager` | lockfile と CI で同一版を使う |

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
| `anyhow` | 1.0.104 | 1.0.104 | 実行ファイル側の最終エラー |

`tokio`、`reqwest`、`sqlx`、`windows` は LF-002 以降で導入する。使う段になってから確認値を取り直す。

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

### 確認値と導入値が異なる理由

**TypeScript**: 公式の `latest` は 7.0.2 だが、`typescript-eslint@8.70.0` の peer 条件は `typescript >=4.8.4 <6.1.0` である。TypeScript 7 を入れると `pnpm lint` の型情報付き検査が成立しない。lint を落とすより型検査を保つ方を優先し、条件を満たす最新の 6.0.3 を導入値にした。`typescript-eslint` が 7 系へ対応した時点で、独立した更新 PR で再評価する。

**Rust**: `docs/04` §2 の候補は 1.98.1 で、開発ホストの `rustup` 既定は 1.98.0 だった。`rust-toolchain.toml` で 1.98.1 を固定し、その版でビルドと検査を実行した。

## 5. 固定の確認方法

```bash
rustc --version          # rust-toolchain.toml の channel と一致する
node --version           # .node-version と一致する
pnpm --version           # package.json の packageManager と一致する
cargo tree --workspace   # Cargo.lock の確定値を確認する
pnpm list --depth 0 -r   # pnpm-lock.yaml の確定値を確認する
```

固定値を変更する PR では、このファイルの確認日、確認値、導入値、選定理由を同じ PR で更新する。
