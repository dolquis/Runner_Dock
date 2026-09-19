---
name: runner-dock-backend-lifecycle
description: "Runner DockのWindows / WSL Backend（NativeWindowsBackend、WslBackend）、Runnerの取得・登録・起動・停止・削除・更新、WSL Guestの長寿命維持、プロセス所有権、資源制約、スリープ・再起動からの復帰を実装・レビュー・検証するときに使用する。docs/07_WINDOWS_WSL_BACKENDS.mdとdocs/05_DOMAIN_STATE.md、ADR-004 / 005の該当節へ誘導し、既存環境の破壊と状態の丸めを防ぐ。「Runnerを起動して」「WSLを管理して」「サービスを止めて」「再起動後に復帰させて」のような依頼で必ず使う。認証だけ、UIの見た目だけ、IPCの型だけの変更には使用しない。"
---

# Runner Dock Backend ライフサイクルの入口

Backend は開発者の PC の既存環境の中で動く。壊した場合に困るのは Runner Dock ではなく、そのユーザーの他の作業である。このスキルは仕様を複製せず、読む節と越えてはならない線だけを示す。

## 読む

1. 触れる操作を先に特定し、`docs/07_WINDOWS_WSL_BACKENDS.md` の該当節だけを読む。「既存環境を壊さない原則」「構築の前提チェック」「保存場所」「Runner取得・登録」「Windows起動」「WSLの起動維持」「systemdの位置づけ」「ディストリビューションの導入」「停止と削除」「資源制約」「スリープ・再起動・障害」から選ぶ。
2. 状態と遷移に触れるなら `docs/05_DOMAIN_STATE.md` の「希望状態」「観測状態」「起動操作の遷移」「停止操作の遷移」「Agent再起動時」「同時実行の意味」「コア不変条件」を読む。
3. 責務の置き場所に触れるなら `docs/03_ARCHITECTURE.md` の「Backendの抽象化」「WSL Guest」「プロセス所有権」「更新責任」を読む。
4. 該当する ADR を読む。ADR-004（WSL を長寿命 Guest で管理）、ADR-005（GitHub 公式 Runner と公式 wrapper を使う）、ADR-012（同時接続数と同時 job 数を混同しない）、ADR-013（Sandbox、Docker、ephemeral は後続）。

## 守る

1. 明示的に取り込んだ対象以外の既存サービス、ディストリビューション、Workflow、秘密情報を変更しない。導入前に前提チェックを行い、変えるものを事前に提示する。
2. `wsl --shutdown`、`wsl --unregister`、サービスのワイルドカード停止、曖昧な Runner 名への「最初の 1 件」フォールバックを通常操作に入れない。対象は一意に解決してから操作する。
3. GUI 終了と Node 停止を同一視しない。逆に、Agent がクラッシュした後も子プロセスが安全に残ると仮定しない。プロセス所有権をどちらが持つかを差分の中で明示する。
4. 希望状態と観測状態を別に持つ。観測状態には取得時刻を添え、取得できなかったときは `Unknown` を保つ。エラーや期限切れを `Offline`、`Idle`、成功へ丸めない。
5. `busy == false` は真偽値の完全一致で判定する。未定義を既定値で埋めない。
6. 同時接続数と同時 job 数を混同しない。資源制約は「資源制約」節の定義に従う。
7. GitHub 公式の Runner と wrapper を使い、独自の常駐・更新機構で置き換えない。アプリ更新と Runner 更新を分ける（`docs/16_OPERATIONS_RELEASE.md`）。

## 検証する

- 差分を `state-invariant-reviewer` に read-only で読ませ、返った指摘を実ファイルで確認する。破壊操作や資格情報に触れるなら `security-boundary-reviewer` も併用する。
- モックで確認したことと Windows / WSL 実機で確認したことを分ける。実機検証は `docs/HUMAN_GATES.md` HG-03 で、`docs/12_TEST_PLAN.md`「証跡フォーマット」に沿って記録する。Runner 登録やサービス操作そのものは HG-02 である。
- 仕様を変えたら `docs/07_WINDOWS_WSL_BACKENDS.md`、`docs/05_DOMAIN_STATE.md`、該当 ADR、`docs/20_TRACEABILITY.md` のうち失効した記述を同じ PR で直す。
