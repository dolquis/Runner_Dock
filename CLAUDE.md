@AGENTS.md

# CLAUDE.md — Claude Code 固有事項

このファイルは Claude Code 固有事項だけを持つ。`AGENTS.md` の再読と要約の再掲をしない。

repo 固有のセルフレビュー項目と検証コマンドは `AGENTS.md` の「検証とセルフレビュー」を正典とし、共通手順は `pre-pr-self-review` に置く。`dolquis/agent-ops` からベンダリングした共有 Skill の本文・`references/` は、この repo で直接編集しない。

## 役割

Claude Code は設計・リサーチ・タスク管理・レビューを主に担当する（`agent:claude-design` / `agent:claude-review`）。Codex Cloud への assign / delegate / mention は、人間 lead の明示許可なしに行わない。

## 日本語文書

日本語の README、設計書、ADR、仕様書、解説の本格的な作成・推敲では `japanese-doc-workflow` を入口にする。短いコメント、コミットメッセージ、コードだけの変更には使わない。

## ターンの終わり方

ターンを終える前に最後の段落を読み返す。計画、質問、次の手順、「次に〜する」で終わっているなら、その作業を今ツールで実行する。可逆で依頼から導かれる操作は許可を求めず進め、破壊的な操作、依頼範囲の変更、`AGENTS.md` が確認を求める操作では止まる。一部が塞がったら残りを仕上げ、外したものと理由を書く。ユーザーが問題を述べ質問している段階では、成果物は評価であって修正ではない。

長いツール連鎖では着手時に 1 行、作業中に短い進捗、最後に単独で読める要約を置く。小さな変更はファイル全体の書き換えではなく該当箇所の編集にする。テストは依頼が求めるか repo が同種の変更で持つ場合にだけ commit し、隣接テストと同規模に収める。

## サブエージェントと Skill の使い分け

- 広域の調査で結論だけ要るときは `Explore`、実装方針の整理は `Plan` を使う。既知のファイルや symbol の 1 件確認は直接検索する。
- Draft PR を作る前と最終報告の前は `pre-pr-self-review` を実行し、あわせて `diff-auditor` で独立した差分レビューを受ける。返された指摘は親が実ファイルで検証する。
- PR 全体のレビュー依頼には `pr-review-toolkit` の agent を使う。`diff-auditor` の代替ではなく、必須ゲートに追加する。
- 領域別の read-only 検証は repo 所有の agent に分担する。認証・token・昇格・破壊操作に触れる差分は `security-boundary-reviewer`、希望状態 / 観測状態や Backend 境界に触れる差分は `state-invariant-reviewer`、IPC・SQLite schema・生成型に触れる差分は `ipc-contract-checker` を使う。build / test など成果物を書く検証は親が実行し、agent には出力を渡す。
- PR の作成は `create-draft-pr` を使う。`commit-commands` の PR 作成コマンドは Draft 規約を保証しないので使わない。

## Agent Teams

`.claude/settings.json` の `env` で `CLAUDE_CODE_EXPERIMENTAL_AGENT_TEAMS` を有効にし、表示は `in-process` とする。対話セッションだけで動き、`-p` や remote / web セッションでは teammate を起動せず通常の subagent として扱う。

- 使うのは、Rust コアと TypeScript / Tauri 契約のように独立した領域を同時に進める実装、または複数の read-only agent を並列に走らせるレビューに限る。直列依存の作業や小修正には使わない。
- lead が統合と最終検証を担う。teammate ごとに担当ファイルを分け、同じファイルや build directory を同時に書かせない。push と PR 作成は lead が直列に行う。
- teammate は teammate を起動できず、`/resume` で復元されない。`.claude/agents/` の `tools`・`model`・本文は teammate にも適用される。

## Advisor（Fable）

複数段階の設計、Backend 抽象や IPC の責務境界、昇格分離や secret 非出力などの重要な不変条件、繰り返す失敗、重要変更の完了前レビューでは、利用可能なら Fable Advisor に相談する。助言は批判的に検討し、最終判断は自分で行う。メインモデルが Fable のときは、上位への相談ではなく同格モデルの独立チェックとして扱う。

typo や軽微で可逆な変更では使わず、Human Gate や Linear 運用の代替にしない。未設定の環境では本節を無視してよい。
