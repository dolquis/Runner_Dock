# 13. 開発ロードマップ

**区分:** 段階計画 / **工数・日程:** 未見積り。技術検証の結果で見積もる

## 1. リリース段階

| 段階 | 目的 | 主な成果 | 出口条件 |
|---|---|---|---|
| M0 / v0.0 PoC | 重要な技術仮説を潰す | workspace、Mock、Windows/WSL寿命、IPC、認証spike | G1〜G4の判断記録がある |
| M1 / v0.1 MVP | 自分のprivate repoで日常利用 | 両Runner構築、一括操作、二重ヘルス、ログ、CLI、ルーティング生成 | 必須FRとG5、安全・配布試験を通過 |
| M2 / v0.2 | 少数repoでの利便性向上 | scope拡張、既存取込、CST差分、資源制約 | 対応機能の権限・変換テストを通過 |
| M3 / v0.3以降 | 隔離・自動化の拡張 | Docker/一時環境、App連携深化、割当制御 | threat model再評価と独立PoC |

M1は二段で判定します。**M1a 個人利用MVP**は、P0要件(FR-001〜017)のうち配布・署名・本体更新に依存する部分(FR-014の本体更新、LF-019)を除く受入条件を満たし、かつ所有者自身のPCとprivate repoでG1〜G5を通過した状態です。技術検証の合格だけではM1aと呼びません。未署名の開発ビルドとして扱い、配布しません。**M1b 公開MVP**はM1aにLF-019/020/022とG6(署名・更新・uninstall・runbook)を加えた状態です。M1aの到達をM1b作業の前提にし、署名や更新基盤の準備を理由に実機検証を遅らせません。

公開MVPと個人用の実験ビルドを区別します。未署名・未検証の試作品は、公開製品の運用手順と同じ位置に置きません。

## 2. M0: 先に確かめること

**Windowsプロセス管理:** 公式Runnerのラッパーと自己更新を壊さず、GUIと独立して管理できるか。

**WSLの寿命:** 長寿命GuestセッションがGUI非表示・アイドル・再接続後も維持されるか。systemdだけに依存していないか。

**認証:** Device Flowとrefreshが配布者共有secretなしで成立するか。対象repoの管理権限とApp installationが正しく交差するか。

**権限:** 非昇格運用、別SIDのIPC拒否、機密の非継承が成立するか。

PoCではUIの完成より、検証結果と棄却理由を重視します。失敗した仮説はADRを更新してからM1へ進みます。

## 3. M1の実装順

```text
LF-001 workspace
   └─ LF-002 domain / protocol / Mock
        ├─ LF-003 Windows spike ── LF-007 Windows provisioning
        ├─ LF-004 WSL spike ───── LF-008 WSL provisioning
        ├─ LF-005 IPC security
        ├─ LF-006 auth spike ──── LF-012 credential lifecycle
        └─ LF-010 Mock dashboard

上記を統合
   → LF-009 lifecycle
   → LF-011 remote health / LF-013 logs / LF-029 metrics
   → LF-014 router / LF-015 Workflow UI / LF-016 CLI
   → LF-017 startup settings / LF-018 removal recovery
   → LF-019 packaging updates
   → LF-020 security QA / LF-021 real integration
   → LF-022 release gate
```

認証spikeの実APIアクセスやRunner登録には所有者が指定したテストrepoを使います。新規repoを勝手にpublicで作らないでください。

## 4. 並行化の方針

Agent Aはcore/protocol、BはWindows、CはWSL、DはUI Mock、EはGitHub API/認証を担当できます。ただしAが最初の契約を確定するまでは、各担当が独自DTOを作らないこと。

DB migration、protocol、生成型、依存lockfileの更新担当を1人にします。機能ごとにworktreeを分け、1タスク1PRを基本とします。並行化の数より、共有契約の変更を早く伝えることを優先します。

Workflow解析は純粋関数中心なので、coreと独立したfixture作成から先行できます。実ファイル書込やGitHub Secrets操作は後から明示承認付きで統合します。

## 5. Go / No-Go

| ゲート | Go条件 | No-Go時の選択肢 |
|---|---|---|
| G1 | Windows起動・停止・更新が検証できた | supervisor方式を見直す。Listener直起動でごまかさない |
| G2 | WSL起動維持と巻込み防止が検証できた | Guest寿命・専用distro方針を変更する |
| G3 | 安全な認証更新が検証できた | 個人PoCはPAT補助。製品UXはADRで再設計 |
| G4 | 最小権限・IPC・秘密・削除境界が通った | 該当機能を公開MVPから止める |
| G5 | テストrepoでWindows/Linux/hosted事前選択成功 | ルーティングを実験機能に留める |
| G6 | 署名・更新・uninstall・runbookが揃った | 個人向け開発ビルドに限定する |

## 6. MVPを膨らませないための制限

一台、一repo、二Backendを最低線とします。macOS、Kubernetes、enterprise、複数PCクラスタ、課金ダッシュボード、一般プラグインAPI、任意コードsandboxをM1へ入れません。

将来の拡張可能性はIDとBackend境界で確保し、まだ使わない抽象層を大量に実装しません。必要になった時点でfixture、ADR、権限、受入試験を増やします。

## 7. 見積り方法

LF-003/004/006の結果をもとに、各Issueへ実装量・検証環境・不確実性を記入します。認証ブローカーやWindows署名の準備を、通常のUI実装と同じ難易度で見積もりません。所要日数はここでは根拠が不足するため固定しません。
