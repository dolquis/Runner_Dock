# Runner Dock 開発開始ドキュメント一式

**製品名:** Runner Dock / **版:** 0.1.0-design / **作成・仕様確認日:** 2026-09-19

Windows PC 1台を、WindowsネイティブとWSL2 UbuntuのGitHub Actions実行環境として構築・運用するための、**新規独自開発プロジェクトの設計基準**です。HomeRunのフォークやコード移植を前提にしません。

> このパッケージは開発用文書です。アプリ、インストーラー、動作確認済みの実装は含みません。採用する設計、確認できた外部仕様、実機検証が必要な仮説を区別しています。製品名はRunner Dockです(2026-09-20決定。商標・既存名との重複確認は製品の配布開始前に行います)。公開ライセンスはMIT OR Apache-2.0です([ADR-016](docs/17_ADR.md))。配布方法は未確定です。タスクIDの接頭辞`LF-`は旧仮称に由来する識別子で、変更しません。

## 最初に読むもの

1. [開発開始指示書](START_DEVELOPMENT.md): 最初のPRと技術検証の範囲。
2. [プロジェクト憲章](docs/01_PROJECT_CHARTER.md)と[要件定義](docs/02_REQUIREMENTS.md): 作るもの・作らないもの。
3. [アーキテクチャ](docs/03_ARCHITECTURE.md)、[セキュリティ](docs/09_SECURITY.md)、[設計判断](docs/17_ADR.md): 実装が守る境界。
4. [バックログ](docs/14_BACKLOG.md): 依存関係付きの作業単位。

AIコーディングエージェントは、リポジトリ直下の [AGENTS.md](AGENTS.md) を常時読みます。Claude Code は [CLAUDE.md](CLAUDE.md) 経由で同じ内容を読み、固有事項を追加で受け取ります。最初に取り組む範囲は [START_DEVELOPMENT.md](START_DEVELOPMENT.md) にあります。文書のみの状態で、記載された未実装コマンドが動くとは扱わないでください。

エージェント向けの Skill、サブエージェント、文書検査器、Linear 運用規約の所在は [.claude/skills/MANIFEST.md](.claude/skills/MANIFEST.md) と [docs/README.md](docs/README.md) にまとめてあります。

## 文書一覧

| 文書 | 内容 |
|---|---|
| [01 プロジェクト憲章](docs/01_PROJECT_CHARTER.md) | 製品価値、対象ユーザー、独自開発方針、非目標 |
| [02 要件定義](docs/02_REQUIREMENTS.md) | FR/NFR、MVP範囲、具体的な受入条件 |
| [03 アーキテクチャ](docs/03_ARCHITECTURE.md) | UI、Agent、Guest、Backendの責務と境界 |
| [04 技術選定](docs/04_TECHNOLOGY_STACK.md) | 現行安定版の確認、選定理由、固定方法 |
| [05 ドメイン・状態](docs/05_DOMAIN_STATE.md) | Node→Backend→Runner、希望状態と観測状態 |
| [06 GitHub認証・API](docs/06_GITHUB_AUTH_API.md) | Device Flow、PAT、権限、期限、API障害 |
| [07 Windows・WSL実装](docs/07_WINDOWS_WSL_BACKENDS.md) | 起動維持、終了、既存環境保護、更新 |
| [08 Workflow支援](docs/08_WORKFLOW_ROUTING.md) | 事前選択、信頼境界、生成例、制約 |
| [09 セキュリティ](docs/09_SECURITY.md) | 脅威モデル、秘密情報、公開PR、破壊操作 |
| [10 IPC・データモデル](docs/10_IPC_DATA_MODEL.md) | メッセージ契約、SQLite、エラー、移行 |
| [11 UI/UX](docs/11_UX_SPEC.md) | 初回導入、ダッシュボード、確認画面、状態表示 |
| [12 テスト計画](docs/12_TEST_PLAN.md) | 単体、契約、Windows/WSL実機、障害注入 |
| [13 ロードマップ](docs/13_ROADMAP.md) | リリース段階、検証ゲート、開発順序 |
| [14 バックログ](docs/14_BACKLOG.md) | Issue化可能な作業と完了条件 |
| [15 開発者ガイド](docs/15_DEVELOPER_GUIDE.md) | 構成、ローカル開発、品質基準、並行開発 |
| [16 運用・リリース](docs/16_OPERATIONS_RELEASE.md) | ログ、更新、署名、復旧、削除、サポート |
| [17 ADR](docs/17_ADR.md) | 設計判断と棄却案、再検討条件 |
| [18 参考実装・未確定事項](docs/18_REFERENCE_REVIEW.md) | HomeRunから学ぶ範囲、既存案の修正、リスク |
| [19 根拠資料](docs/19_SOURCES.md) | 一次資料のURL、確認日、用途 |
| [20 追跡表・文書検証](docs/20_TRACEABILITY.md) | 要件→タスク→テストと文書整合性検査 |
| [実装タスク雛形](templates/IMPLEMENTATION_TASK.md) | 人間・AI共通の作業指示テンプレート |
| [Human Gate](docs/HUMAN_GATES.md) | AI 単独で確定しない人間判断の登録簿 |
| [Linear 運用規約](docs/linear-conventions.md) | 共有コアと Runner Dock の Project Delta |

## 採用方針の要約

```text
Tauri 2 / React / TypeScript
              │ 許可済みの型付きIPC
              ▼
非昇格のWindows User Agent
  ├─ NativeWindowsBackend → GitHub公式Runner
  ├─ WslBackend → 長寿命WSL Guest → GitHub公式Linux Runner
  └─ GitHub API / 認証 / ローカル設定

UIに同梱した独立TSライブラリ → Workflow解析・テンプレート生成
ファイル保存は許可済みの製品操作を経由
```

本体はRust、非同期処理はTokio、ローカル設定はSQLiteを軸にします。バージョンは[技術選定](docs/04_TECHNOLOGY_STACK.md)の確認結果を起点に、最初のビルド成功時に固定します。

## 先に共有しておく制約

- GitHubのRunner選択は自宅PCのGUIだけでは変わりません。Workflow側の対応が必要です。[S01](docs/19_SOURCES.md#s01)
- `online` / `busy` の照会は予約ではありません。照会直後の停止・競合は残ります。[S01](docs/19_SOURCES.md#s01)[S02](docs/19_SOURCES.md#s02)
- WSLのsystemdサービスだけに起動維持を任せません。明示的なGuestプロセスの寿命を管理します。[S04](docs/19_SOURCES.md#s04)
- GitHub Appの認可コードフローにPKCEを加えても、公開デスクトップアプリから`client_secret`が不要になるとは限りません。本案はDevice Flowを先に検証します。[S08](docs/19_SOURCES.md#s08)[S09](docs/19_SOURCES.md#s09)
- 私用PCの同一ユーザーで走る任意コードを、UI、暗号化保存、WSLだけで安全なサンドボックスに変えることはできません。[S07](docs/19_SOURCES.md#s07)[S19](docs/19_SOURCES.md#s19)

## 文書の読み方

**決定**は初期実装の基準、**提案**は本書作成者の設計案、**検証ゲート**は実機で確認するまで製品機能として約束しない項目です。外部仕様は`Sxx`から根拠へ移動できます。文書間で矛盾した場合は、要件とセキュリティを優先してADRを更新します。

日付は文書の基準日です。将来の実装時には、依存関係の最新パッチ・GitHub API・WSLの挙動を再確認してください。

## ライセンス

このリポジトリの文書とソースコードは、次のいずれかを利用者が選択できるデュアルライセンスで提供します。

- MIT License ([LICENSE-MIT](LICENSE-MIT))
- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))

明示的な別段の表明がない限り、このリポジトリへ意図的に提出されたコントリビューションは、Apache-2.0の定義に従い、追加の条件なしに上記のデュアルライセンスで提供されたものとして扱います。製品名「Runner Dock」とロゴの利用権はライセンスに含みません。
