---
name: runner-dock-linear-ops
description: "Runner DockのLinear Issue（Dev / Runner Dock Project）を起票・更新・状態遷移・ラベル付け・作業後コメントするとき、およびPRとIssueの対応付け（DEV-番号、Refs / Fixes、In Review → Merged → Done）を扱うときに使用する。docs/linear-conventions.mdの該当節へ最短で誘導し、状態の誤用、Codex起動トークンの再生産、自動Done化を防ぐ。「Issueを作って」「Linearを更新して」「Doneにして」「ラベルを付けて」「作業ログを残して」のような依頼で必ず使う。Linear運用規約そのものの改訂やGitHub PRの作成だけには使用しない。"
---

# Runner Dock Linear 運用の入口

Linear は状態・進捗・優先度・担当の正典であり、規約の正典は `docs/linear-conventions.md` である。このスキルは規約を複製せず、操作ごとに読む節と、AI が越えてはならない線だけを示す。Git、PR、共通セルフレビューは、すでに適用されている `AGENTS.md` に従う。

## 参照資料を読む

1. 操作に対応する節だけを `docs/linear-conventions.md` から読む。全文を読み込まない。Runner_Dock 固有のラベルと Project 名は §13 Project Delta にある。
2. Linear の操作は利用可能な Linear 連携（Claude Code では Linear MCP）で行う。連携が無い環境では、実行すべき変更を Issue ID・状態・ラベル・本文の形で人間へ引き渡し、実行済みと報告しない。

## 操作を分類する

1. 操作を、起票、既存 Issue の更新、状態遷移、ラベル・relation の整備、作業後コメント、PR との対応付け、Project 記録のいずれかに分類する。
2. 依頼が Codex Cloud への assign / delegate / mention を含む場合は、人間 lead の明示許可が Issue にあるかを先に確認する。チャットでの「Codex に投げて」は許可コメントではない。無ければ候補ラベル付けと実行指示文の下書きまでに留める。
3. 状態遷移の前に Issue の現状態、ラベル、PR の参照形式（closing キーワードか非クローズ参照か）を読む。Done を含む場合は Done の条件を先に確認する。

## 操作する

1. 起票は Agent Task Format に沿い、`repo:Runner_Dock`、対象の `area:rd-*`（定義は §13）、`kind:*`、`type:*`、次の役割を示す `agent:*` を付ける。人間専任の判断は `gate:human-required` を付け、`agent:*` を省略してよい。
2. `area:rd-*` は `rd-base` / `rd-agent` / `rd-backend` / `rd-github` / `rd-ui` / `rd-workflow` / `rd-security` / `rd-test` から選ぶ。迷ったら触れるファイルの所在で決め、複数にまたがるなら主たる 1 つに絞る。
3. 依存を本文に書いたら、同時に `blockedBy` relation を張る。
4. 作業後コメントは Agent Run Report Format に沿い、仕様を複製せず GitHub へリンクする。
5. PR 本文には対応する `DEV-<番号>` と、由来する `LF-` タスク ID を書く。`LF-` はローカル識別子であり、Linear の状態の代わりに使わない。検証メモ待ちや Human Gate を含む Issue では `Fixes #` を使わず非クローズ参照にする。
6. 状態は「open PR があれば In Review、マージ済みなら Merged、検証メモを記載してから Done へ明示遷移」の順で動かす。Windows / WSL 実機検証を要する Issue は、`docs/HUMAN_GATES.md` HG-03 の証跡が揃うまで Done にしない。PR を持たない Human Gate Issue は Todo のまま置く。
7. Linear のコメント、Issue 本文、テンプレートのいずれにも、Codex を起動する mention トークンをリテラルで書かない。

## 報告する

- 操作した Issue ID、変更した状態・ラベル・relation、残したコメントの要点を列挙する。
- 実行できなかった操作（連携が無い、許可が無い、Done 条件未達）は理由を明記し、次のオーナーを示す。
- repo docs へ進捗や状態を複製しない。状態の正典は Linear である。
