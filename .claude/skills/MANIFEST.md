# Skill マニフェスト

この repo が持つ Skill の区分、origin、取り込み revision を記録する。編集規約は `AGENTS.md`「Human Gate と編集禁止対象」と `.claude/skills/AGENTS.md` にある。

`.claude/skills/` は Claude Code、`.agents/skills/` は Codex が読む。両ツリーは本文と `references/` を同期し、ハーネス固有 frontmatter だけ差異を許す。乖離は `python3 scripts/docs-lint.py --category mirror` が検出する。

## 共有 Skill（origin: `dolquis/agent-ops`）

この repo で本文と `references/` を編集しない。改訂は origin で行い、`scripts/vendor-shared-skills.sh` で配り直す。repo 側の編集は silent fork になる。

取り込み revision: `ff4cbb4`

| Skill | 用途 | 両ツリー |
|---|---|---|
| `japanese-tech-writing` | 日本語文書の執筆・推敲 | あり |
| `argument-gap-edit` | 論理飛躍と段落構成の修正 | あり |
| `japanese-doc-workflow` | 日本語文書作業の入口 | あり |
| `doc-governance` | README / AGENTS / CLAUDE / docs の変更 | あり |
| `pre-pr-self-review` | stage / commit / push / PR 前のセルフレビュー | あり |
| `create-draft-pr` | Draft PR の作成 | あり |
| `doc-coauthoring` | 文書の共同執筆ワークフロー | Claude のみ |

`doc-coauthoring` は第三者 Skill（Apache-2.0）で、Codex 非対応のツールとサブエージェントを前提とするため `.agents/` 側に置かない。この非対称は `.docs-lint.toml` の `[mirror] claude_only` に登録してある。`LICENSE` と `NOTICE.md` を同梱し、出典とライセンス表記を保持する。

## repo 固有 Skill（origin: この repo）

Issue の範囲で変更できる。変更したら両ツリーを同じ PR で同期する。

| Skill | 発動対象 | 指す正典 |
|---|---|---|
| `runner-dock-linear-ops` | Issue 起票、状態遷移、ラベル、PR との対応付け | `docs/linear-conventions.md` |
| `runner-dock-security-boundaries` | 認証、token、昇格、シェル起動、破壊操作、診断成果物 | `docs/09_SECURITY.md`、`docs/06_GITHUB_AUTH_API.md` |
| `runner-dock-backend-lifecycle` | Windows / WSL Backend、Runner の登録・起動・停止・更新 | `docs/07_WINDOWS_WSL_BACKENDS.md`、`docs/05_DOMAIN_STATE.md` |
| `runner-dock-ipc-contracts` | IPC メッセージ、SQLite schema と migration、生成型 | `docs/10_IPC_DATA_MODEL.md` |

## サブエージェント

`.claude/agents/` と `.codex/agents/` に本文一致で置く。いずれも read-only で、書き込み・commit・push・Linear 操作を行わない。build / test のように成果物を書く検証は親が実行し、agent には出力を渡す。

| agent | 役割 | origin |
|---|---|---|
| `diff-auditor` | 設計意図から切り離した独立差分レビュー | `dolquis/agent-ops` の `templates/` |
| `security-boundary-reviewer` | 信頼境界、資格情報、昇格、破壊操作の確認 | この repo |
| `state-invariant-reviewer` | 希望状態 / 観測状態の分離とコア不変条件の確認 | この repo |
| `ipc-contract-checker` | IPC 契約と SQLite schema の両側整合の確認 | この repo |

`diff-auditor` は origin の骨子をそのまま写したうえで、read-only を frontmatter でも担保するため `disallowedTools` を足してある。
