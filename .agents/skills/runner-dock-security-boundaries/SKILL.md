---
name: runner-dock-security-boundaries
description: "Runner DockのGitHub認証（Device Flow、PAT、tokenの3用途分離）、昇格を伴う処理、シェル起動、Runner登録・削除、ログと診断ZIPへの出力、公開PRの実行先判定を実装・レビュー・検証するときに使用する。docs/09_SECURITY.mdとdocs/06_GITHUB_AUTH_API.md、ADR-003 / 006 / 007 / 010の該当節へ誘導し、token漏洩、昇格の拡大、シェル注入、破壊操作の常用化を防ぐ。「認証を実装して」「tokenを保存して」「Runnerを登録して」「診断ZIPを作って」のような依頼で必ず使う。UIの見た目だけ、状態遷移だけ、文書だけの変更には使用しない。"
---

# Runner Dock セキュリティ境界の入口

Runner Dock は開発者の PC 上で GitHub の資格情報を保持し、OS のサービスとプロセスを操作する。境界が一箇所崩れると、token 漏洩か既存環境の破壊のどちらかになる。このスキルは仕様を複製せず、読む節と越えてはならない線だけを示す。

## 読む

1. 触れる境界を先に特定し、`docs/09_SECURITY.md` の該当節だけを読む。「保護対象と信頼境界」「保証しないこと」「脅威と対策」「権限設計」「TauriとIPC」「Secretの取り扱い」「Workflowの安全性」から選ぶ。
2. 資格情報に触れるなら `docs/06_GITHUB_AUTH_API.md` の「認証方式の決定」「資格情報を3用途に分ける」「必要権限」「トークン失効・削除」を読む。
3. 既存環境を変える操作に触れるなら `docs/07_WINDOWS_WSL_BACKENDS.md` の「既存環境を壊さない原則」「停止と削除」を読む。
4. 該当する ADR を読む。ADR-003（GUI と非昇格 Agent の分離）、ADR-006（Device Flow）、ADR-007（ローカル管理 token と Workflow 監視 token の分離）、ADR-010（秘密と特権を UI から分離）。

## 守る

1. token、Authorization ヘッダー、Runner 登録応答、登録 token を、ログ、診断 ZIP、`Debug` 出力、例外メッセージ、telemetry、clipboard、エラー文字列へ出さない。新しい出力経路を足すたびに、この一覧に対して確認する。
2. ローカル管理 token と Workflow 監視 token を、値・保存先・scope のすべてで分ける。片方の失効がもう片方に波及しない構造にする。
3. 日常運用の経路に昇格を要求しない。昇格は初回設定処理へ分離し、その処理が何を変えるかを事前に提示する。
4. 外部文字列（repo 名、Runner 名、ラベル、パス、API 応答）を `cmd /c` や `bash -c` へ連結しない。引数配列か固定ラッパーを使い、Windows の引数処理もテストする。
5. UI から Runner、OS シェル、token、DB を直接操作しない。型付きの許可リストを経由する。汎用 `exec` 相当の IPC を足さない。
6. `wsl --shutdown`、`wsl --unregister`、サービスのワイルドカード停止、曖昧な Runner 名への「最初の 1 件」フォールバックを通常操作の経路に入れない。
7. 公開 PR と同一 repo 内 PR を含む未信頼コードは、既定で self-hosted へ送らない。生成 Workflow だけを境界として扱わず、実行側にも防御を置く。
8. GitHub API のエラーや期限切れを `Offline`、`Idle`、成功へ丸めない。`Unknown` と鮮度を保持する。

## 人間へ渡す

GitHub App 作成、PAT 発行と scope 決定、Secrets や課金設定の変更、Runner 登録、サービス停止、署名、公開リリースは `docs/HUMAN_GATES.md` の HG-01、HG-02、HG-05 である。AI は最小 scope 案と理由、影響範囲、復旧手順まで整理して引き渡し、実行しない。secret の値そのものを Issue、PR、コミット、ログへ書かない。

## 検証する

- 差分を `security-boundary-reviewer` に read-only で読ませ、返った指摘を実ファイルで確認する。
- 境界や権限を変えたら、`docs/09_SECURITY.md`、`docs/06_GITHUB_AUTH_API.md`、該当 ADR、`docs/HUMAN_GATES.md`、`docs/20_TRACEABILITY.md` のうち失効した記述を同じ PR で直す。
- 実機でしか確認できない項目（実際の token 発行、Runner 登録、サービス操作）は「未実行」と明記し、成功と書かない。
