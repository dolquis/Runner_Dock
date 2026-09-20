# AGENTS.md — Runner Dock エージェント規約

原則として日本語で回答する。本ファイルは、この repo で常時適用する最小の共通契約である。詳細は作業に必要な正典の該当 ID・節だけを読む。

## 1. プロジェクトと正典

Runner Dock は、Windows PC 1 台を Windows ネイティブと WSL2 Ubuntu の GitHub Actions 実行環境として構築・運用するデスクトップアプリである。Tauri 2 + React の UI、非昇格の Windows User Agent、Backend 抽象（`NativeWindowsBackend` / `WslBackend`）で構成し、本体は Rust、非同期は Tokio、ローカル設定は SQLite を軸にする。HomeRun のフォークや移植を前提にしない新規独自実装である。`docs/` の索引は `docs/README.md` が持つ。

| 対象 | 正典 |
|---|---|
| 機能・非機能要件、受け入れ条件 | `docs/02_REQUIREMENTS.md` |
| 責務境界、Backend 抽象、モノレポ構成 | `docs/03_ARCHITECTURE.md` |
| 状態遷移、コア不変条件 | `docs/05_DOMAIN_STATE.md` |
| 脅威モデル、信頼境界、権限設計 | `docs/09_SECURITY.md` |
| IPC 契約、SQLite schema、エラー契約 | `docs/10_IPC_DATA_MODEL.md` |
| 設計判断 | `docs/17_ADR.md` |
| 要件 → タスク → テストの追跡 | `docs/20_TRACEABILITY.md` |
| 状態・進捗・優先度・担当・計画 | Linear の Runner Dock / Self-hosted Runner Manager MVP |
| Linear 運用 | `docs/linear-conventions.md` |
| 安全・倫理・ライセンスの人間判断 | `docs/HUMAN_GATES.md` |

矛盾時は受け入れ基準について `docs/02_REQUIREMENTS.md` を優先する。文書中の構成図には repo に無い要素も載っている。図を既存実装として扱わず、動くコマンドは `README.md`「開発コマンド」の範囲に限る。

## 2. 作業開始と変更方針

- 作業開始時、進捗確認時、PR 前は `git status -sb` を確認する。remote の状態を判断する前は `git fetch origin` を行い、切り替えずに `origin/main` との差を見る。
- 確認前に依頼、承認済み会話、Issue、spec、ADR、既存契約を照合する。それでも外部挙動、IPC schema、受入条件が決まらなければ具体案と影響を示して確認し、回答に依存しない作業は進める。契約内の実装や、仕様へ戻す修正は根拠を示して続行する。
- 割り当てられた LF タスクの依存は `docs/14_BACKLOG.md` で確認する。`LF-` は旧仮称由来のローカル識別子で、Linear の `DEV-<番号>` とは別系統である。
- 最小差分を基本とし、無関係なリファクタリング、整形、rename を混ぜない。

## 3. 条件付きで読む資料

全資料を一律に全文読込しない。`rg`、見出し、要件 ID、Issue のリンクから対象を絞り、必要な節を前後の文脈とともに読む。

| 作業 | 読むもの |
|---|---|
| 通常の実装・修正 | Issue が示す要件 ID、設計 spec、関連実装・テスト |
| 認証・token・昇格・破壊操作・診断成果物 | `runner-dock-security-boundaries` |
| Windows / WSL Backend、Runner の登録・起動・停止・更新 | `runner-dock-backend-lifecycle` |
| IPC メッセージ、SQLite schema、生成型、エラー契約 | `runner-dock-ipc-contracts` |
| Linear / PR 状態変更 | `runner-dock-linear-ops` |
| 日本語の本格的な文書作業 | `japanese-doc-workflow`。短いコメントやコミット文には使わない |

Skill の行は、その Skill が指定する参照まで含む。表に無い作業は §1 と `docs/README.md` から辿る。


## 4. 調査・実装ツール

- 文字列検索・局所編集は Read / Edit / `rg`、symbol の宣言・実装・参照・診断・rename・削除はファイル数によらず Serena を優先する。Serena は初回に `initial_instructions` → `get_current_config` で対象の絶対パスと言語を確認し、使えなければ `rg` と実ファイルで代替する。一時 worktree を登録しない。
- GitHub 操作は利用可能な connector を優先し、無ければ `gh` を使う。
- `.codegraph/` があり CodeGraph が利用可能で、構造や影響範囲が不明なら先に使う。Context-Mode は大量の文書・diff・ログの整理に使う。どちらも要約だけで完了判断せず、実ファイルと最新 diff で確認する。公開 API、IPC schema、SQLite migration、認証、権限、安全設計、外部連携では参照元と consumer を先に洗い出す。

### ローカルサブエージェント

ローカル分担の共通規則は `docs/linear-conventions.md`「Codex Execution Policy」のローカル分担段落を正典とする。read-only の領域別レビューは repo 所有 agent へ分担し、成果物を書く検証は親が実行して出力だけ渡す。

## 5. 実装上の不変条件

### 5.1 設計境界

1. HomeRun をフォークしない。ソースや UI 資産を無断で移植せず、参照元と知見を記録する。
2. UI は Runner、OS シェル、GitHub token、DB を直接操作しない。操作は型付きの許可リスト経由にする。
3. 日常運用の Agent・Runner を管理者または Linux root で動かさない。昇格は初回設定処理へ分離する。
4. GUI 終了と Node 停止を同一視しない。Agent クラッシュ後に子プロセスが安全に残るとも仮定しない。
5. 既存サービス、ディストリビューション、Workflow、秘密情報は、明示的に取り込んだ対象以外を変更しない。
6. `wsl --shutdown`、`wsl --unregister`、サービスのワイルドカード停止、曖昧な Runner 名への「最初の 1 件」フォールバックを通常操作に入れない。
7. GitHub API エラーや期限切れを `Offline`、`Idle`、成功へ丸めない。`Unknown` と鮮度を保持する。
8. ローカル優先ルーティングは事前判定である。キュー待ちや実行中ジョブの無停止移行を実装済みと表現しない（ADR-008）。
9. `busy == false` は真偽値の完全一致で判定する。`jq '.busy // true'` は使わない。
10. 公開 PR、同一 repo 内 PR を含む未信頼コードは、既定で self-hosted へ送らない。生成 Workflow だけをセキュリティ境界と考えない。

### 5.2 実装ルール

書き方と共有境界の同時編集は `docs/15_DEVELOPER_GUIDE.md`「コーディング規約」「並行開発の境界」が正典で、ここには安全側の不変条件だけを置く。

- `core` を Tauri、WSL コマンド、GitHub HTTP クライアントに依存させない。実行・時計・秘密情報・保存先はインターフェースで注入する。
- 外部文字列を `cmd /c`、`bash -c` へ連結しない。固定ラッパーや引数配列を使い、Windows の引数処理もテストする。
- token、Authorization ヘッダー、登録応答、Runner 資格情報をログや診断 ZIP へ含めない。MCP の secret は設定ファイルへ直書きせず、`${ENV_VAR}` 経由で渡す。
- `unsafe` は OS 連携に閉じ込め、必要理由、不変条件、対象 API の資料、テストを付ける。
- Rust stable を使い、`rust-toolchain.toml` と `Cargo.lock` を固定する。nightly や RC は承認済み ADR がある場合だけとする（ADR-011）。

### 5.3 文書の不変条件

- 要件変更を伴う実装では、影響する要件定義、設計 spec、ADR、`docs/20_TRACEABILITY.md` を同じ PR で同期する。新規または rename した文書は `docs/README.md` に索引する。
- repo docs は定義だけを持ち、状態を持たない。見出しで進捗・完了を主張せず、状態語（語彙は `python3 scripts/docs-lint.py --print-words`）を地の文へ書かず、進捗表や課題一覧、README の TODO を作らない。状態の正典は Linear である（§1）。
- コード参照に行番号や実測件数を書かない。ファイル名・型名・関数名までに留める。

## 6. 検証とセルフレビュー

- 文書と Skill を変更したら `python3 scripts/docs-lint.py --baseline .docs-lint-baseline.json`、`python3 scripts/docs-lint.py --category mirror`、`python3 scripts/check_agent_instruction_size.py` を実行する。CI 必須ゲートの再現は `pre-commit run --all-files` である。
- 実装が入ったら `docs/15_DEVELOPER_GUIDE.md`「初期化後に定義する開発コマンド」の最小ゲートを使う。定義前の段階でそこに書かれたコマンドが動くと扱わない。
- repo 内ファイルを変更したら、最終報告前と stage、commit、push、PR 更新前に `pre-pr-self-review` を使う。使えない環境では `origin/main` との全差分と未追跡ファイル、secret の混入、編集境界、文書同期を確認する。
- モックで動いたことと Windows / WSL 実機で動いたことを分ける。未実行のテストは「未実行」と明記し、成功と書かない。実機でしか確認できない項目を無断で skip しない。
- 変更起因の問題は修正して検証を再実行する。無関係な既存問題は勝手に直さず報告する。未実施・失敗・既存失敗は理由と影響を明記する。ツール診断や CI の成功を §8 の Human Gate の証拠に置き換えない。

## 7. Linear・ブランチ・PR

- 実装は Linear Issue 起点とし、`dolquis/dev-<番号>-<slug>` で 1 Issue = 1 branch = 1 Draft PR とする。
- `main` へ直接 push / force push しない。PR は `dolquis/Runner_Dock` の `main` 向け Draft とし、base / head / compare 範囲と同一 head の既存 PR を確認する。作成は `create-draft-pr` が利用可能なら使う。既存 Ready PR を Draft に戻さない。通常のマージ方法はノーマルマージとする。
- PR 本文に目的、理由、Linear Issue、対応する LF タスク ID と FR / ADR、実行コマンドと結果、未確認事項、リスク、ロールバック方法、Documentation impact、Human Gate を記載する。失敗の再現テストを先に追加する。
- PR マージ時は Linear を Merged とし、検証メモ後に Done へ明示遷移する。状態・ラベル・ミラー規約は `docs/linear-conventions.md` を読む。
- セッション内で直さない問題は Linear の当該 Project に起票する。`agent:codex-*` は候補ラベルであり、Codex Cloud の assign / delegate / mention には人間の明示許可が要る。

## 8. Human Gate と編集禁止対象

- `docs/HUMAN_GATES.md` の判断を AI 単独で確定しない。GitHub App 作成、PAT 発行、Secrets や課金設定の変更、Runner 登録、サービス停止、branch protection、コード署名、公開リリース、実機検証が対象である。論点・選択肢・リスクを整理して人間へ引き渡す。文書やソースの変更依頼は、外部アカウントやこの PC の設定への変更を許可しない。
- CI の自己管理機能を開発中の Agent 自身に無条件で適用しない。自分を停止・更新するテストは隔離されたテスト用環境で実施する。
- `docs/linear-conventions.md` §1〜§12、`scripts/docs-lint.py`、`scripts/check_agent_instruction_size.py` とそのテスト、`.claude/hooks/post-edit-docs-lint.py`、ベンダリングした共有 Skill の本文・`references/` をこの repo で編集しない。変更は origin（`dolquis/agent-ops`）で行い配布し直す。§13、`.docs-lint.toml`、repo 所有のドメイン Skill は Issue の範囲で変更できる。
- repo 所有 Skill は `.agents/skills/` と `.claude/skills/`、repo 所有 subagent は `.codex/agents/` と `.claude/agents/` を同じ PR で本文一致のまま同期する。`doc-coauthoring` の Claude 専用は意図的な非対称である。
- secret、credential、token、`.env`、生成物、ローカル設定、`.codegraph/`、`.serena/`、`.context-mode/`、`.ctx/`、キャッシュ、セッションログを編集・index・commit しない。
