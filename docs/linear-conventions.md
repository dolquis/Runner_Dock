<!--
  SHARED CORE — Agent / Linear 運用規約（管制塔モデル）
  この「共有コア」は全リポジトリで同一内容をミラーする。
  個別 repo で直接編集しない。編集は origin（後述）で行い、各 repo へ伝播する。
  version: 0.10  updated: 2026-09-04
  改訂履歴は origin の git log を正典とし、本ヘッダには追記しない。
  上書き型で日付を持たない器へ履歴を手書きすると、§7.2 が禁じている
  手書きキャッシュそのものになるためである。
  origin(編集の起点・単一正典): dolquis/agent-ops/linear-conventions.md（このファイル）
  各 repo の docs/linear-conventions.md は本ファイルのベンダリングコピー + §13 Delta。
  プロジェクト固有の差分は各 repo の「Project Delta」節（本ファイル末尾）に置く。
-->

# Linear 管制塔モデル 運用規約（Shared Core）

Dev チーム配下の全プロジェクトで共通の、Linear 運用ルール。
**詳細仕様・ロードマップ・本規約の正典は GitHub repo docs。Linear ドキュメントはそのミラー。**

このファイルは、全 repo で同一内容をミラーする「共有コア」（§1–§12）と、各 repo だけが書き換える「Project Delta」（§13）で構成される。共有コアは origin で 1 回編集し、各 repo へ同一内容を伝播する（個別 repo で直接編集しない）。

---

## 1. 役割分担（control tower）

- **Linear = 管制塔**: 状態・優先度・進捗・親子関係・依存・担当エージェント・「次に AI へ渡す Issue」のルーティング。
- **GitHub = 正典(source of truth)**: 実装規約・ビルド/テスト手順・詳細仕様・ロードマップ、そして本運用規約。
- **Repository docs**: `AGENTS.md` / `README.md` / `docs/*` / `ROADMAP`（または `WORKFLOW.md`）。

原則: Linear に仕様を複製しない。詳細は GitHub 正典を参照する。

---

## 2. エージェント分業

| ラベル | 役割 |
| -- | -- |
| `agent:claude-design` | 仕様整理・設計・タスク分割・レビュー観点作成 |
| `agent:claude-review` | 実装後レビュー・整合性確認・リスク洗い出し |
| `agent:codex-impl` | 実装担当 |
| `agent:codex-pr-review` | PR 差分レビュー担当 |

Issue には「次の AI 役割」を示す `agent:*` を 1 つ付ける。ただし `gate:human-required` または旧 `type:human-gate` の人間専任タスクは `agent:*` を省略してよい。

---

## 2.1 Codex Execution Policy（Codex 実行ポリシー）

対象: Codex Cloud（"Codex for Linear"）。Linear で Issue を Codex に assign / delegate する、コメントで mention トークン（`@`+`Codex`）を付ける、または triage rule で自動 delegate すると起動する。ローカルの Codex App は Codex チャット（Linear 管轄外）から起動し、Linear のラベルでは起動しない。

- `agent:codex-impl` / `agent:codex-pr-review` は **ルーティング（候補）ラベル**であり、Codex Cloud の実行を許可しない（滑走路前の待機列）。
- Claude は Codex 候補 Issue の作成・分割・ラベル付け・関連付け・整理と、実行指示文の下書きまで行ってよい。ただし Codex への assign / delegate / mention は **行わない**。
- Codex Cloud の実行には人間 lead の明示許可（Issue コメント）が必要。Claude / エージェントはいかなる Linear コメント / Issue 本文 / テンプレートにもリテラルな mention トークン（`@`+`Codex`）を再生産しない（無害化する）。承認後に実際の mention で起動するのは人間 lead のみ。
- triage rule による Codex 自動 delegate は使わない。
- 実行したら Codex Run Record（§6）に approval / Codex task link / branch / commit / PR / validation / remaining risk を記録する。
- 無許可で Codex Cloud が動いた場合はインシデントとして扱う: delegate を解除して Issue を候補へ戻し、GitHub に branch / PR が到達していないか確認し、Issue に記録する。

ローカル分担の共通規則は本段落を正典とし、AGENTS・Skill・tooling runbook から本段落を参照し、それぞれ工程固有の補足を持つ。承認されたローカル作業内の分担は Cloud 起動と区別する。品質改善または時間短縮が見込める独立した調査・実装・レビューには、機構が利用可能な枠内で積極的に分担する。利用できない場合は同じ検証範囲を親が担当する。目的、対象・非対象、書込み境界、成果形式、検証、PR 本数・Issue 順序・完了条件を渡し、全履歴を無条件に複製しない。親が統合と最終検証を担い、共有ファイル・build directory の競合を防ぎ、共有 Serena の対象変更を直列管理する。小修正や直列依存のみの仕事は分割しない。この分担は Cloud 起動、新しいユーザー所有タスク作成、仕様変更、Human Gate の追加承認を代行しない。

### 実行許可フォーマット（人間 → Issue コメント）

- Issue / Repo / Scope（Acceptance Criteria のみ）
- Allowed output: summary only | branch only | draft PR | PR（repo の PR 規約に従う。Draft PR 必須の repo では draft PR までとする）
- Human gate: none | required before Done
- Prohibited: 無関係な refactor / スコープ変更 / main への直接 push / human-gate 判断の変更

このコメントがある場合に限り、人間が Codex への delegate / mention を行う。

---

## 3. ワークフロー状態

標準フロー（Linear のステータス名が完全一致しなくても、この順序で解釈する）:

1. **Backlog / Todo** — 未整理または未着手
2. **Design** — Claude Code で仕様整理・実装方針作成
3. **Implement** — Codex で実装
4. **Review** — Claude Code または Codex で差分レビュー（Linear の In Review。open な PR がある）
5. **Merged** — PR マージ済み・検証メモ待ち（Done 判定前の滞留を可視化する）
6. **Done** — repo 固有の完了条件を満たし、作業結果・検証・PR・ドキュメント影響確認が記録済み

### 3.1 状態に意味を重ねない

In Review が「PR レビュー待ち」「マージ済み検証メモ待ち」「人間ゲート待ち」を兼ねると、キューの内訳が Linear 単体では分からず、監査のたびに GitHub の open PR 実態と突合することになる（2026-08-31 監査では In Review 18 件のうち open PR を持つ課題は 1 件だった。DEV-923）。次のように使い分ける。

- **In Review** は open な PR（Draft 含む）を持つ課題だけに使う。open PR が無い課題を In Review に置かない。
- **Merged** は PR がマージ済みで、検証メモの記載と Done への明示遷移（§7.1.3）が済んでいない課題に使う。
- **人間ゲート課題**（`gate:human-required` の人間専任 Issue）は PR を持たないため In Review / Merged に置かず、Todo のまま人間の着手を待つ。キューは §10 の Needs Human Verification ビューが担う（started 系の状態に置くと、着手できないまま cycle 集計に乗り続ける）。
- **`type:tracking`** は子 Issue の進行中は In Progress を維持する。子 PR のマージは束ねの完了ではないため、In Review / Merged に置かない。

### 3.2 Cycle は commitment（scope の上限と、外す条件）

Linear は cycle の終了時に未完了 Issue を次の cycle へ自動で繰り越す。入れた分は消えず、残った分がそのまま次の scope になるため、Cycle を「やりたいこと置き場」として使うと scope は単調に増え、burndown は平坦なままになる（2026-07-27 監査では Cycle 8 が scope 96 件・completed 0 件で終わっていた。DEV-666）。Cycle には **その週に状態が動く見込みのあるものだけ**を入れる。

数値（件数・Project 数・持ち越し回数）は人間 lead が決め、cycle の実績で見直す。

1. **投入は commitment として決める**。その週に状態が動く見込みのあるものだけを入れる。上限は 20 件。
2. **同時 In Progress は 8 件まで**（`type:tracking` と `gate:human-required` を除く）。上限に達したら、新規着手より In Review / Merged の解消を先に行う。
3. **`type:tracking` を Cycle に入れない**。束ね Issue は 1 週間で閉じないため、scope と burndown の両方を歪める。追跡は Project とラベルで行い、Cycle には子の implementation Issue を入れる。
4. **外部要因で止まっているものは Cycle から外す**。第三者の返答、他者のリリース、ベンダーの回答を待つ Issue は、こちらの帯域を消費しないまま未完了として積み上がる。
5. **人間ゲートは、その週に人間が処理すると決めたものだけ入れる**。`gate:human-required` は AI 側から動かせないため、入れっぱなしにすると滞留の芯になる。
6. **`[Recurring] ... control tower audit`（§11）を Cycle に入れない**。週次トリガで回し、scope に数えない。
7. **同時に進める Project は 3 つまで**。残りは Cycle の外に置き、Project の Status Update に止めている理由と再開条件を書く（§12）。
8. **PR がマージされたら In Progress に留めない**。マージ → Merged（Merged state を持たない間は In Review）→ 検証メモ記載 → Done（§7.1.3）。`Part of` / `Refs` のような closing キーワードでない参照ではこの遷移が自動で起きないため、手で動かす。検出は `scripts/linear-audit.py`（§11）が持つ。
9. **止めるときにアーカイブを使わない**。アーカイブした Issue は通常ビュー・Cycle・Project の集計から消えるため、状態の正典が Linear であるという前提が崩れる。取り下げは `Canceled`、続けるが今は動かさないものは Backlog へ戻し、いずれも理由をコメントに書く（team に停止用の状態を設けるならそれを使う）。未完了のままアーカイブされた Issue の検出は `scripts/linear-audit.py` が持つ。
10. **2 cycle 続けて持ち越された Issue は Cycle から外す**。外すのは取り下げではなく triage への差し戻しで、Project・Priority・ラベルは保持したまま Backlog / Todo に置く。繰り越し以外の出口が無いと、Cycle は commitment ではなく堆積物になる。
11. **同じ所見が 2 回続けて出たら扱いを変える**。ラベル・メタデータ級は監査セッションがその場で直して記録する。構造・方針級は人間へ通知し、通知しないまま次回へ持ち越さない。監査が同じ指摘を書き足すだけの装置になると、検出はできても処置されない状態が固定する。

Cycle から外した Issue は Project・Priority・ラベルを保持するため、Backlog / Todo からいつでも再投入できる。外すことと取り下げることを混同しない。

---

## 4. ラベル分類体系（コロン式に統一）

ラベル prefix はコロン式に統一する。技術領域 `area:` も複数付与するが、Linear 上では label group 化せずフラットなラベルとして運用するため、コロン式でも複数選択できる（Linear のラベルグループ排他制約は明示的なグループにのみ働き、`area:foo` のような prefix 付きフラットラベルには働かない。2026-06-03 決定、後日コロン式へ統一）。Phase 4 完了までは各 repo の Delta / `AGENTS.md` が定める active label を優先する。

| プレフィックス | 用途 | 値 |
| -- | -- | -- |
| `repo:` | 対象 GitHub リポジトリ（実 repo 名をそのまま反映） | 各 1 つ必須（Phase 4 完了までは旧 repo ラベルも有効） |
| `area:` | 技術領域 | 1 つ以上（複数付与可。Linear ではフラットなラベルとして運用。旧 `area_*` は Phase 4 で付け替え） |
| `agent:` | 次の AI 役割（§2） | 原則 1 つ（人間専任タスクを除く） |
| `type:` | Issue の役割 | `tracking` / `implementation` / `review` |
| `gate:` | 人間ゲート（横断フラグ） | `human-required` |
| `kind:` | 変更カテゴリ | `feature` / `improvement` / `bug` / `docs` |
| `Migrated` | 由来フラグ（他サービス起票） | 任意 |

ルール:
- `area:` は複数選択可能にするため Linear label group にせずフラットなラベルとして運用する（例 `area:converter-core`）。`repo:` は実 GitHub repo 名をそのまま使い区切りを正規化しない。
- **`area:docs` は repo 横断の共有 area**（ドキュメント・ガバナンス・規約の整合）。特定 repo の技術領域に属さない文書系 Issue に付与する。repo 固有の技術領域 area（例: azooKey `area:settings`（設定アプリ / schema）/ `area:build`（CMake・CTest・CI・コード署名・MSIX パッケージング））は各 repo の Delta / `AGENTS.md` で定義する。
- **人間ゲートは `type:` ではなく `gate:human-required`（横断フラグ）で表す。** 例: 人間確認が必要なレビュー Issue は `type:review` + `gate:human-required`。旧 `type:human-gate` は本規約で廃止。移行期は旧ラベルが残る Issue があるため Phase 4 で `gate:human-required` へ付け替える（読むときは両対応）。
- **変更カテゴリは `kind:*` を正典とする**（`feature` / `improvement` / `bug` / `docs`）。旧 `Feature` / `Improvement` / `Bug` / `enhancement` / `documentation` は Phase 4 まで移行互換として扱い、Phase 4 で物理退役（ラベル削除は設定画面で手動）する。`Migrated` は由来フラグとして存続。
- **Phase 4 完了までの移行互換**: 旧 `repo_*` / 旧 `area_*` / 旧カテゴリラベルのみが付いた Issue も、Ready / Missing Metadata / 週次監査では欠落扱いしない。新規作成・更新時は repo の Delta または既存 `AGENTS.md` の現行ラベルを優先し、Phase 4 後に `repo:` / `area:` / `kind:*` へ収束する。

---

## 5. Agent Task Format（Issue 記述）

Fields: Background / Goal / Repository / Area label / Files / Expected behavior / Plan / Done criteria / Owner / Handoff notes

Claude が Issue を作成・更新・分割するときは、本文冒頭に **Agent Handoff** ブロックを置く:

```md
## Agent Handoff

- Type:
- Agent:
- Repo:
- Goal:
- First files to inspect:
- Protected areas:
- Required validation:
- Expected PR size:
- Blocks:
- Blocked by:
- Codex safety: `agent:codex-*` はルーティングのみ。Claude は Codex へ delegate / assign / mention しない（実行は人間 lead のみ。mention トークンは候補段階で書かない）。
```

Issue タイプ（`type:`）:
- `tracking` — 進捗管理・子 Issue 集約・順序整理。直接実装しない。
- `implementation` — 1 PR で完了可能な実装・修正・テスト追加。
- `review` — 設計レビュー・PR レビュー・整合性確認。

次に AI へ渡す Issue の並び（最大 3 件）は description に書かない。Project は Project Status Update に、tracking Issue は `Status snapshot YYYY-MM-DD` 見出しのコメントに置く。いずれも日付つきの追記型で古さを測れる器である（§7.2 / §12）。

---

## 6. Agent Run Report Format（作業後コメント）

作業後に Linear Issue へ残すログ形式（仕様ではなく作業ログ）:

- **Agent**: Claude Code / Codex / ChatGPT / other
- **Read**: 確認した正典（AGENTS / roadmap / README / 連携 GitHub Issue / 関連 docs）
- **Changed**: 変更ファイル、または Linear のみの変更
- **Validation**: test / lint / build / 手動確認、スキップ時は理由
- **Findings**: リスク・ブロッカー・follow-up
- **Next**: 次アクションと次オーナー

Codex Cloud を実行した場合は、追加で **Codex Run Record** を残す:

- Execution approved by / approval comment
- Codex task link / branch / commit / PR
- Validation / known limitations
- Human gate required / next reviewer

短く保ち、仕様はコピーせず GitHub へリンクする。

---

## 7. Definition of Ready / Done

### Ready
- Project が正しく設定されている。
- `repo:*` ラベルがある（Phase 4 完了までは repo 固有の旧 repo ラベルも有効）。
- 関連する `area:*` が 1 つ以上ある（Phase 4 完了までは repo 固有の旧 `area_*` ラベルも有効）。
- 次の AI 役割を示す `agent:*` がある。ただし `gate:human-required` または旧 `type:human-gate` の人間専任タスクは `agent:*` を省略してよい。
- 可能なら GitHub Issue / PR リンクが添付されている。
- Goal と done criteria が明確。
- 実行順序が重要な場合、ブロッカーが relation で表現されている。

### Done
- 作業結果が記録されている。
- 検証結果が記録されている（スキップ時は理由）。
- GitHub 正典に対するドキュメント影響を確認済み。
- 可能なら関連 PR / GitHub Issue がリンクされている。
- 必要な follow-up Issue が作成/リンクされている。
- `gate:human-required` または旧 `type:human-gate` の Issue は人間確認が取れている。
- repo 固有 Delta / `AGENTS.md` / `WORKFLOW.md` が追加ゲート（PR マージ、検証メモなど）を要求する場合、それを満たしている。

Done は「Linear 上で運用的に完了」を意味し、GitHub docs のリリース基準を置き換えない。

---

## 7.1 Design / Gate Split（設計層と人間ゲートの分離）

`agent:claude-design` / `agent:claude-review` の AI 設計作業と `gate:human-required` の人間判断が両方絡む Issue は、原則として **設計 Issue** と **人間ゲート Issue** の 2 件に分離する。1 件に同居させると、設計 PR のマージで Issue 全体が誤って Done 化し、未達の人間ゲートを飛び越える（管制塔の状態が実態と乖離する）。

### 7.1.1 分離テスト（分割するか否か）

人間が必要とする作業が、**AI セッションでは生み出せず、かつ同一レビュー内で人間が即座に記録もできない**もの（実機・実データ計測 / 購入・契約 / 外部アカウント開設 / 法務・ライセンス確定 / 署名値設定 / 本番デプロイ 等）を含むなら **分割必須**。

人間の入力が「AI が用意した決定ブリーフの『決定』欄を埋める」程度で、**同一サイクル内に完了**するなら、分割せず単一 Issue のままその場でゲートをクリアしてよい（決定内容・合意日を検証メモに記録）。

### 7.1.2 分割後の規格

| 項目 | 設計 Issue | 人間ゲート Issue |
| -- | -- | -- |
| タイトル | 元のまま | `Human Gate: <決定内容>` で開始（roadmap コード併記可: 例 `D-04-A`） |
| `agent:*` | `agent:claude-design` / `claude-review` | 付けない（人間専任。§2 / §7 Ready で免除） |
| `gate:human-required` | **付けない** | 付ける |
| `type:` | `review` 等 | `review` |
| `repo:` / `area:` / `kind:` | 通常どおり | 通常どおり |
| 関連付け | — | 設計 Issue へ `related`、所属 tracking / epic を parent、リリース律速なら下流へ `blocks` |
| Done 条件 | 調査 / spec / 雛形 / ハーネスが PR マージ + 検証メモで確定 | 人間判断・実機検証 + 決定 / 計測値の記録（検証メモ） |

設計 Issue は純 AI スコープになるため、Done 化時に `gate:human-required` を残さない（クリア済みなら除去、未クリアなら Done にしない）。

### 7.1.3 自動 Done の防止（設定・運用）

事故の根本原因は、Linear–GitHub 連携が PR マージ / ブランチ名連動で Issue を Done 化し、人間ゲートを飛び越える点にある。次の多層で防ぐ:

1. **連携設定（採用）**: チームの GitHub 連携で「PR マージ時の遷移先」を **Done ではなく Merged** にする（`Merged` は team `Dev` に作成済み。2026-09-01・DEV-924）。最終 Done は必ず人間 / Claude の明示操作とする。これにより設計 PR のマージは Merged で止まり、人間ゲートの取りこぼしが構造的に起きない。マージ済みの課題がレビュー待ちの課題と混ざらないため、In Review は「open PR あり」を保ち続ける（§3.1。この設定変更は人間 lead が Linear 側で行う）。**ただし自動遷移が働くのはクローズ系リンク（`Fixes` / `Closes` / `Resolves`）で参照した課題に限る。** 非クローズ参照の課題は連携が動かさないため、-4 の手動処置が唯一の経路になる（2026-09-01 に PR 10 本をマージした際、`Part of` で参照された DEV-928 / DEV-929 が In Progress のまま残った）。
2. **closing キーワードの使い分け**: 設計 PR は設計 Issue のみを `Fixes DEV-<design>` で閉じる。人間ゲート Issue は closing キーワードで参照せず `Ref DEV-<gate>` / `Part of DEV-<gate>` のみとし、PR リンクは attachment で手動付与する。 GitHub ミラー Issue も同様に、人間ゲート / 検証メモ待ちの Issue では `Fixes #<N>`（マージで GitHub Issue をクローズ → Linear 同期で Done 化し Merged を迂回する）を避け、`Refs #<N>` 等の非クローズ参照にする。 この書き分けの副作用として、`Part of` / `Refs` で参照した課題は merge 自動遷移も止まる。自動 Done を防ぐのと引き換えに Merged への遷移も自動では起きないため、-4 で手動処置する。
3. **分割の徹底**: §7.1.1 に該当する Issue は分割し、auto-close が人間ゲート Issue に当たらないようにする。
4. **検証メモのフロー化**（§7.2 遷移時記録原則の適用例）: PR のマージを検知したセッション（マージを実行した人間から引き継いだエージェント・PR 監視エージェント・直後に該当 repo で作業するセッション）は、その場で検証メモ（確認したテスト名・CI ジョブ名を含む）を Issue にコメントし、`gate:human-required` が無ければ Done へ明示遷移する。**非クローズ参照の課題では Merged への遷移そのものも手動で行う**（-2 のとおり連携は動かさない）。自動遷移を前提にしない。遷移時にやることは §7 Done チェックリストに集約してある。検証メモを週次監査でまとめて書く運用は Merged の滞留を生む（DEV-662 で 18 件、DEV-925 で 11 件が一括処理になった）ため、監査での記入は取りこぼしの回収に限る。

本節の遷移規約（PR マージ→Merged、Done は明示遷移）は、各 repo の `AGENTS.md` / `docs/GITHUB_LINEAR_MAPPING.md` / `docs/WORKFLOW.md` 等のライフサイクル要約より**優先**する。要約側が「PR マージ→Done」と記す場合は本節に読み替え、可能なら要約側も更新する。

---

## 7.2 記録の鮮度

tracking Issue の進捗欄と Project description は、子 Issue の状態を人手で写した**手書きキャッシュ**である。無効化の仕組みがないため、元データが動くたびに黙って実態とずれる（DEV-775: 進捗欄が 27 日放置され、description は全面更新の 2 時間後に子 3 件の Done 遷移で再び陳腐化し、検出から修正まで 15 日かかった）。全面更新の 2 時間後に腐る以上、更新頻度を上げても解決しない。書ける内容を絞り、腐る面そのものを減らす。

**原則**: Linear の状態から**導出できる記述を description に書かない**。現在は生データ（sub-issue リスト・blocked-by リレーション・mention チップ）に語らせる。導出できない意図・構造・過去の事実は書いてよい。

| 区分 | 例 | 判定 |
| -- | -- | -- |
| 構造と意図 | 分解の意図、spec アンカー、正典の所在、依存の理由 | 書いてよい（腐らない） |
| 日付つき完了記録 | 「Done（PR #89、2026-08-31）」 | 書いてよい（過去の事実） |
| 状態依存の記録 | Current focus、Next AI Tasks、Health | description に書かず、**日付つき追記型の器**へ置く（下記） |
| 導出可能な現在 | 状態名（`In Review` / `Merged` 等）、「着手可」「〜待ち」「残 N 件」 | 書かない |
| 状態スナップショット | 「進捗（YYYY-MM-DD 時点）」節 | description に置かず、`Status snapshot YYYY-MM-DD` 見出しのコメントへ積む |

「導出可能な現在」は上の語彙の列挙ではなく、**Linear を引けば分かる記述すべて**を指す（列挙は例示）。コメントは日付を持つ過去ログとして読まれるため腐らない。description が腐るのは「現在」を名乗るからである。

**器で担保する**: 状態依存の記録をゼロにはできない。次に何を渡すかは Linear から導出できないからである。消せない以上、鮮度は規律ではなく**器**に持たせる。Project Status Update と Issue コメントは日付つきの追記型で、最終更新からの経過日数を API が返す。つまり古さを機械判定できる。description は上書き型で日付を持たないため、古さを測れない。測れない面に「毎回更新する」という規律を課しても守られなかった（実測: `Next checkpoint` の 63 日超過、Project の Next AI Tasks への Done / Canceled 混入 27 件）。

- Project の状態依存記録 → **Project Status Update**（書式と周期は §12）。
- tracking Issue の状態依存記録 → **`Status snapshot YYYY-MM-DD` 見出しのコメント**。

**遷移時記録原則**: 器が日付を持つぶん、個々の遷移を追いかけて直す義務は課さない。読者は日付を見て古さを判断できるためである。義務は**周期内に 1 本、全節そろったエントリを積むこと**に置く（§12）。節を欠いたエントリを積むと「最新を読めば足りる」が崩れ、読者が履歴を遡ることになる。周期の逸脱は週次監査（§11）が機械判定で拾う。

**到達性**: 現在を description から外すぶん、状態は Linear を引いて得る。作業開始時に対象 Issue の子・blocked-by と §10 のキューを取得してから動く。

---

## 8. Navigation Rules

- Linear リンクはナビゲーションと計画にのみ使う。
- Tracking item は小さな作業項目を束ねる。
- Order/blocking マーカーは実際の順序にのみ使う。
- Reference マーカーは緩い関連に使う。
- Same/duplicate マーカーは真に同一の作業にのみ使う。

GitHub docs remain canonical.

---

## 9. Do Not Do

- GitHub roadmap の詳細を Linear docs にコピーしない。
- Linear docs を仕様(spec)として扱わない。
- 人間ゲート Issue を手動確認なしでクローズしない。
- 実機・署名など人間判断が必要な作業を AI 判断だけで Done にしない。
- Migrated 作業で GitHub Issue リンクを省略しない。
- Linear 作業から README に進捗表/TODO を増やさない。
- Claude から Codex へ assign / delegate / mention しない。実行は人間 lead が明示許可コメント後に自ら行う（Claude は実行指示文の下書きまで）。
- Claude / エージェントはいかなる Linear コメント / Issue にもリテラルな Codex mention トークン（`@` + `Codex`）を再生産しない（無害化する）。承認後に実際の mention で起動するのは人間 lead のみ。
- triage rule で Codex を自動 delegate しない。
- `agent:claude-*`（design / review）と `gate:human-required` を同居させたまま PR で auto-close される構成にしない。分離テスト（§7.1.1）に該当するなら 2 件に分割する。
- Done 化時に `gate:human-required` を残さない（人間判断がクリア済みなら除去、未クリアなら Done にしない）。

---

## 10. Control Tower Views（推奨ビュー）

各プロジェクトで以下のフィルタビューを用意する（Project でスコープ）:

- **Ready for Claude Design**: `agent:claude-design` + Backlog/Todo
- **Ready for Claude Review**: `agent:claude-review` + Todo/In Review
- **Codex Candidate Queue**: `agent:codex-impl` + Todo + delegate なし + 非ブロック（候補。実行はしない）
- **Codex Review Candidate Queue**: `agent:codex-pr-review` + Todo/In Review + delegate なし（候補。実行はしない）
- **Delegated to Codex**: delegate = Codex（暴発・実行中・実行済みの監査用。Candidate と必ず分離する）
- **Needs Human Verification**: `gate:human-required`（+ not Done。旧 `type:human-gate` も読む）
- **In Review**: status In Review（open PR のレビュー・マージ待ち。open PR の無い課題が現れたら §11 で再分類する）
- **Merged / Needs Verification Memo**: status Merged（マージ済み・検証メモ待ち。滞留は検証メモ債務として §11 で点検する）
- **Missing Metadata**: repo / area / agent ラベル欠落、または Migrated Issue の GitHub リンク欠落（移行期の旧 repo/area ラベルと `gate:human-required` / 旧 `type:human-gate` 人間専任タスクの `agent:*` 免除を考慮）

Codex Candidate（`agent:codex-*` 候補）と Delegated to Codex（delegate 済み・実行）は絶対に混ぜない。`delegate = Codex` が見えたら必ずレビュー対象にする。

ビューはレーダー画面であって仕様ではない。曖昧なら GitHub docs と連携 GitHub Issue を見てから動く。

---

## 11. Recurring Control Tower Audit（週次・統一チェックリスト）

各プロジェクトに `[Recurring] Linear control tower audit — <PROJECT>` を 1 件持つ
（`type:review` + `agent:claude-review` + `repo:*`。この recurring audit Issue 自体は `area:*` 免除）。チェック項目:

- [ ] Project 未設定の Issue
- [ ] `repo:` ラベル欠落（Phase 4 完了までは repo 固有の旧 repo ラベルも有効）
- [ ] `area:*` ラベル欠落（Phase 4 完了までは repo 固有の旧 `area_*` ラベルも有効。recurring audit Issue 自体を除く）
- [ ] `agent:` ラベル欠落（`gate:human-required` または旧 `type:human-gate` の人間専任タスクを除く）
- [ ] Migrated なのに GitHub リンク欠落
- [ ] 人間確認が要るのに `gate:human-required` 欠落
- [ ] Tracking Issue で子が未リンク
- [ ] 実行順序を表さなくなったブロッカー
- [ ] Done なのに検証ノート欠落
- [ ] `agent:claude-*` と `gate:human-required` が同居した Issue（Design / Gate 分割漏れ。§7.1）
- [ ] Done の設計 Issue（`agent:claude-*` 付き）に `gate:human-required` が残っている（分割後に除去すべき stale ラベル。人間ゲート Issue 自体が検証メモ付きで Done なのは正常）
- [ ] 人間ゲート Issue が PR の auto-close 対象になっている（closing キーワードで参照されている）
- [ ] `type:tracking` Issue が Codex 実行候補（`agent:codex-*` + Todo・delegate なし・非ブロック）として Candidate Queue（§10）に現れている（tracking は束ね専用で直接実装しない。§5 / §8。実行は子 implementation Issue へ降ろす）

Codex safety checks:

- [ ] 人間 lead の明示許可なく Codex へ delegate された Issue がない
- [ ] 明示許可なく Codex mention トークン（`@`+`Codex`）を含むコメント / Issue 本文 / テンプレートがない
- [ ] `agent:codex-*` をルーティング（候補）ラベルとしてのみ扱っている
- [ ] Codex 実行開始後に Todo へ放置された delegate 済み Issue がない
- [ ] Codex 完了タスクに task / PR / commit リンク・検証・残リスクが記録されている
- [ ] 人間ゲート Issue が人間確認なしで Done になっていない
- [ ] ブロック中の Codex 候補が Ready として表示されていない
- [ ] 依存する設計 Issue（`agent:claude-design` / `agent:claude-review`）が未完了（not Done）のまま、その下流 implementation Issue に Codex 実行許可（§2.1 の許可コメント / delegate）が出ていない（設計固定前の実装着手＝仕様の雰囲気決定を防ぐ。§2.1 / §3 / §7.1）

機械判定項目（レーン衛生・記録の鮮度・期日）は **origin の `scripts/linear-audit.py` が正典**として持つ。監査セッションはまずこれを実行し、`CONFIRMED` を処置してから、人力を上下の判断が要る項目に充てる。`REVIEW` はヒューリスティック（description の文言と実状態の突合）で過検出を前提とするため、採否は人が決める。規約側に項目を列挙し直さない（インシデントごとに足し続けると、規約自身が無効化機構のない手書きキャッシュになる）。

Design / Implementation spec-first checks（設計 §2 / §7.1 の spec-first 分業の担保）:

- [ ] `agent:codex-impl` のフィーチャー（`kind:feature`。Phase 4 完了までは旧 `Feature` / `enhancement` も対象）が、対応する `docs/*-spec.md` 節（または roadmap の該当マイルストーン節）で当該サブ課題の難所（IPC payload / JSON schema・境界値・アルゴリズム・責務境界）を確定する前に In Progress 以降へ入っていないか（未着手の課題だけでなく、既に In Progress の課題も対象）。spec が未確定のまま実装着手していないか。
- [ ] 専用 `agent:claude-design` 課題を持たないフィーチャーでも、tracking / 比較レポート / roadmap 節 / `docs/*-spec.md` のいずれかで難所が上流確定され、実装課題からアンカー参照されているか。
- [ ] 波（wave）分割されたフィーチャーで、先行波の spec だけが書かれ後続波が未確定のまま実装着手していないか（just-in-time spec が波ごとに守られているか）。

Rule: Linear のルーティングのみを点検する。GitHub docs が正典。

---

## 12. Project の記録（description と Status Update）

Project の記録は 2 つの器に分ける。**腐らないものを description に、状態依存を Status Update に置く**（§7.2）。

### description（上書き型・日付なし）

次の 4 つの H2 だけを置く（順序もこの通り）。状態は書かない。

```md
## Operating contract

Lead: <name>
Codex safety: `agent:codex-*` は候補ラベルのみ。Claude は Codex へ delegate / assign / mention しない（実行は人間 lead のみ）。
Current focus / Next AI Tasks / Next checkpoint / Health は最新の Project Status Update を参照する（§7.2）。

## Human Gate

* <DEV-xx> <判断の内容> は人間確認必須。（状態は書かない。キューは §10 の Needs Human Verification ビュー）

## Canonical docs

正典: <REPO>/docs/linear-conventions.md, <REPO>/docs/<WORKFLOW or ROADMAP>.md

## Stage map

<ステージ名と定義だけを置く。達成状態・残件・進行中の Issue は書かない（正典は repo の roadmap。§1 / §7.2）>
```

Operating contract の最終行は Status Update への**ポインタ**である。参照先の構造を指すだけで状態を含まないため腐らない。

### Status Update（追記型・日付つき）

**14 日に 1 本以上**積む。各エントリは次の 3 節をすべて持たせる。1 節でも欠けると「最新を読めば足りる」が崩れ、読者が履歴を遡ることになる。

```md
## Current focus

<いま何を優先するかと、その理由。1〜3 文>

## Next AI Tasks

1. DEV-xx <内容>
2. DEV-xx <内容>
3. DEV-xx <内容>   （最大 3 件・すべて not Done）

## Next checkpoint

<YYYY-MM-DD>。<その日に判定する内容>
```

Health は Linear の health フィールドで設定し、本文に重ねて書かない。

### マイルストーン命名
- 1 プロジェクト内では 1 つのトークン体系に統一する（`Stage N` / `P N` / `MVP-N` のいずれか）。
- 数字を綴り字にしない（`Stage Three` ではなく `Stage 3`）。
- 各マイルストーンは Stage map のステージに対応させる。

---

## 13. Project Delta（Runner_Dock 固有）

本節は `dolquis/Runner_Dock` が所有する。§1〜§12 の共有コアは `dolquis/agent-ops` が origin で、`scripts/vendor-linear-conventions.sh` が本節より前だけを差し替える。本節をこの repo で書き換えても origin へは戻らない。

### 13.1 識別子

- PROJECT_NAME: Runner Dock / Self-hosted Runner Manager MVP
- REPO: `dolquis/Runner_Dock`
- REPO_LABEL: `repo:Runner_Dock`
- BRANCH: `dolquis/dev-<番号>-<slug>`
- CANONICAL_DOCS: `docs/02_REQUIREMENTS.md`、`docs/03_ARCHITECTURE.md`、`docs/05_DOMAIN_STATE.md`、`docs/09_SECURITY.md`、`docs/10_IPC_DATA_MODEL.md`、`docs/17_ADR.md`、`docs/20_TRACEABILITY.md`

### 13.2 area ラベル

| ラベル | 範囲 |
|---|---|
| `area:rd-base` | repo 基盤、ガバナンス、CI、エージェント環境、ツールチェーン |
| `area:rd-agent` | 非昇格 Windows User Agent、プロセス所有権、常駐と終了 |
| `area:rd-backend` | NativeWindowsBackend、WslBackend、Runner 取得・登録・起動・停止・更新 |
| `area:rd-github` | GitHub 認証（Device Flow、PAT）、API client、権限、障害の扱い |
| `area:rd-ui` | Tauri フロントエンド、React、ダッシュボード、確認画面、状態表示 |
| `area:rd-workflow` | Workflow Assistant、事前ルーティング、テンプレート生成、Workflow 解析 |
| `area:rd-security` | 脅威モデル、信頼境界、secret の扱い、昇格分離、破壊操作の抑止 |
| `area:rd-test` | 単体・契約テスト、Windows / WSL 実機検証、障害注入、性能測定 |

`kind:*`、`type:*`、`agent:*`、`gate:human-required` は team `Dev` の共通ラベルをそのまま使う。

### 13.3 段階の対応

Linear の milestone は `docs/13_ROADMAP.md`「リリース段階」に対応させる。ロードマップ側に段階を増やしたら、同じ PR で milestone 名を揃える。進捗そのものは Linear が正典で、`docs/13_ROADMAP.md` には書かない。

### 13.4 LF タスク ID との併存

`docs/14_BACKLOG.md` の `LF-` 接頭辞は旧仮称由来のローカル識別子で、変更しない（`README.md`）。Linear Issue ID（`DEV-<番号>`）とは別系統である。Issue 本文で由来となった `LF-` を参照してよいが、`LF-` を Linear の状態の代わりに使わない。

### 13.5 Delta として repo 側が持つ文書

- `docs/HUMAN_GATES.md`（人間判断の登録簿）
- `docs/20_TRACEABILITY.md`（要件 → タスク → テストの追跡表）
- `templates/IMPLEMENTATION_TASK.md`（人間・AI 共通の作業指示テンプレート）
