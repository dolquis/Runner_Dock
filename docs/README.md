# docs 索引

Runner Dock の設計文書の一覧である。文書ごとの役割だけを置き、進捗や状態を持たない。状態の正典は Linear である（`AGENTS.md` §1）。

新規または rename した文書はこの索引へ追加する。編集規約は `docs/AGENTS.md` にある。

## 製品と要件

| 文書 | 役割 |
|---|---|
| [01 プロジェクト憲章](01_PROJECT_CHARTER.md) | 製品価値、対象ユーザー、独自開発方針、非目標 |
| [02 要件定義](02_REQUIREMENTS.md) | FR / NFR、MVP 範囲、受入条件 |
| [13 ロードマップ](13_ROADMAP.md) | リリース段階、検証ゲート、開発順序 |
| [14 バックログ](14_BACKLOG.md) | LF タスクと完了条件、依存関係 |

## 設計

| 文書 | 役割 |
|---|---|
| [03 アーキテクチャ](03_ARCHITECTURE.md) | UI、Agent、Guest、Backend の責務と境界 |
| [04 技術選定](04_TECHNOLOGY_STACK.md) | 現行安定版の確認、選定理由、固定方法 |
| [05 ドメイン・状態](05_DOMAIN_STATE.md) | Node → Backend → Runner、希望状態と観測状態 |
| [06 GitHub 認証・API](06_GITHUB_AUTH_API.md) | Device Flow、PAT、権限、期限、API 障害 |
| [07 Windows・WSL 実装](07_WINDOWS_WSL_BACKENDS.md) | 起動維持、終了、既存環境保護、更新 |
| [08 Workflow 支援](08_WORKFLOW_ROUTING.md) | 事前選択、信頼境界、生成例、制約 |
| [09 セキュリティ](09_SECURITY.md) | 脅威モデル、秘密情報、公開 PR、破壊操作 |
| [10 IPC・データモデル](10_IPC_DATA_MODEL.md) | メッセージ契約、SQLite、エラー、移行 |
| [11 UI/UX](11_UX_SPEC.md) | 初回導入、ダッシュボード、確認画面、状態表示 |
| [17 ADR](17_ADR.md) | 設計判断と棄却案、再検討条件 |

## 品質と運用

| 文書 | 役割 |
|---|---|
| [12 テスト計画](12_TEST_PLAN.md) | 単体、契約、Windows / WSL 実機、障害注入 |
| [15 開発者ガイド](15_DEVELOPER_GUIDE.md) | 構成、ローカル開発、品質基準、並行開発 |
| [16 運用・リリース](16_OPERATIONS_RELEASE.md) | ログ、更新、署名、復旧、削除、サポート |
| [20 追跡表・文書検証](20_TRACEABILITY.md) | 要件 → タスク → テストと文書整合性検査 |

## 根拠と運用規約

| 文書 | 役割 |
|---|---|
| [18 参考実装・未確定事項](18_REFERENCE_REVIEW.md) | HomeRun から学ぶ範囲、既存案の修正、リスク |
| [19 根拠資料](19_SOURCES.md) | 一次資料の URL、確認日、用途 |
| [Human Gate](HUMAN_GATES.md) | AI 単独で確定しない人間判断の登録簿 |
| [Linear 運用規約](linear-conventions.md) | 共有コア（§1〜§12）と Runner_Dock の Project Delta（§13） |
| [docs 編集規約](AGENTS.md) | このディレクトリのローカル規則。[CLAUDE.md](CLAUDE.md) は Claude Code 向けに同じ規約を import する |
