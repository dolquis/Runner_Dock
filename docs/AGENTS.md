# docs/ instructions

- README、AGENTS、CLAUDE、`docs/` を変更するときは `doc-governance` を使う。
- spec や roadmap は対象 ID・見出しを `rg` で特定し、必要な節から読む。全体監査が必要な場合を除き、一律に全文を読み込まない。
- コード変更で仕様、責務境界、fallback、ログ、設定、ユーザー可視挙動が変わる場合は、対応する `docs/0*`〜`docs/2*` の節と `docs/20_TRACEABILITY.md` を同じ PR で更新する。
- 状態、進捗、行番号付きコード参照、変動するテスト件数を恒常文書へ複製しない。状態の正典は Linear である。
- 新規または rename した文書は `docs/README.md` に索引する。一回限りの記録は完了基準を満たしたら `docs/archive/` に置く。
- ADR を追加するときは `docs/17_ADR.md` へ追記し、既存 ADR の番号を再利用しない。決定を覆すときは元の ADR を残し、再検討条件を書く。
- `python3 scripts/docs-lint.py --baseline .docs-lint-baseline.json` を実行し、ベースラインからの増加がないことを確認する。日本語文書を新規作成または大きく推敲するときは `japanese-doc-workflow` を入口にする。
