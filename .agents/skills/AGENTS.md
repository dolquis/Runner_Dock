# Skill instructions

- repo 固有 Skill の作成・更新には `skill-creator` を使い、共有 Skill は `dolquis/agent-ops` を正典として product repo で直接改訂しない。
- repo 固有 Skill は `.agents/skills/` と `.claude/skills/` の本文・`references/` を同期し、ハーネス固有 frontmatter だけ差異を許す。
- Skill の references から「AGENTS.md を再読する」指示を除く。AGENTS.md はセッション開始時に適用済みである。ツール利用は「利用可能なら」の条件付きで書く。
- `description` は「いつ使い、いつ使わないか」を具体的に書く。発動精度はここで決まる。
- 変更した各 Skill に `quick_validate.py` を実行し、`python3 scripts/docs-lint.py --category mirror` で両ツリーの乖離を確認する。
