# 18. 参考実装、既存案の修正、未確定事項

**確認日:** 2026-09-19 / **調査範囲:** 公開README、ARCHITECTURE.md、LICENSE、公式プラットフォーム資料

## 1. HomeRunから確認できたこと

公開READMEにはデスクトップUIと背景daemonを分けたRunner管理が説明されています。ARCHITECTURE.mdには、Rust daemonを中心にGUI/TUIを接続し、公式Runnerを子プロセスとして起動する方式、Dockerでの実行、状態監視、ログ・メトリクスが記載されています。[S28](19_SOURCES.md#s28)[S29](19_SOURCES.md#s29)

一方、確認したARCHITECTURE.mdはmacOS・Unix socket中心の記述を含み、READMEのWindows対応説明と詳細の更新範囲が一致していません。そのため「すべての記載が現在の実装と完全一致」とは扱いません。今回はコード全体の監査、実行試験、保守性評価までは行っていません。

| 学べる内容 | 本プロジェクトでの扱い |
|---|---|
| UIとdaemonの分離 | 同じ問題への有効な分解例として参照。APIや実装は独自設計 |
| 公式Runnerの利用 | GitHubの実行エンジンは再実装せず、構築・運用へ集中 |
| 状態、ログ、メトリクス | Local/Remote/Freshnessを分けた独自状態モデルへ |
| 認証・初回登録UX | 独自GitHub Appと権限境界を検証 |
| native/containerの区別 | Node→Backend→RunnerとしてWindows/WSLを初期要件化 |

HomeRunのLICENSEはMITです。ただし、本方針はコード取り込みではなく新規実装です。自分のプロジェクトの公開ライセンスまで自動的にMITへ決まるわけではありません。[S30](19_SOURCES.md#s30)

「世の中に他に存在しない」「WSL機能が絶対ない」「競合にない機能だから需要がある」とまでは判断していません。独自性と利用価値の仮説は、試用者の導入成功率・操作負担・継続利用で確かめます。

## 2. 過去の案から補正した重要事項

| 過去の単純化された案 | この文書一式の基準 |
|---|---|
| `wsl ... /bin/true`とsystemdだけで常駐 | WSL維持を保証しないため、長寿命Guestを実機検証する [S04](19_SOURCES.md#s04) |
| `systemctl is-active`出力を部分一致で判定 | `inactive`を`active`と誤判定しない。終了コードと完全一致を使用 |
| 最初に見つかったRunner serviceを操作 | exact ID・所有情報で対象を限定。未特定なら停止/削除しない |
| 終了時に`wsl --shutdown` | 原則Guest/Runnerだけ停止。他distroは止めない [S06](19_SOURCES.md#s06) |
| GitHub App + PKCEならsecret不要 | code交換にsecret要件があるためDevice Flowを主候補にする [S08](19_SOURCES.md#s08)[S09](19_SOURCES.md#s09) |
| `jq '.busy // true'`でidleを判定 | falseもfallbackされるため不適切。真偽値を厳密検査 |
| API照会すれば確実に自宅へ割当て | 照会は予約ではなく競合が残る [S01](19_SOURCES.md#s01)[S02](19_SOURCES.md#s02) |
| GitHubが自動的にhostedへ再配置 | Workflow事前選択が必要。実行中の透明な切替ではない [S01](19_SOURCES.md#s01) |
| public PRを分岐すればPCが安全 | 防御の一層。privateでも同一ユーザー実行・依存コードのリスクは残る [S19](19_SOURCES.md#s19) |
| `.wslconfig`がUbuntu Runner専用の上限 | WSL2全体への設定。他distroへの影響と再起動を説明 [S05](19_SOURCES.md#s05) |
| Nodeの同時job数を簡単に1へ固定 | listener数、queue、予約、scopeを区別し、未実装の保証をしない |
| 最新versionとしてVite 8.1を固定 | 確認日の公式support表を基に8.3系を初期候補とした [S17](19_SOURCES.md#s17) |

これらは会話をそのまま実装すると起こり得る問題を減らす修正です。動作検証の代わりではありません。

## 3. リスク登録簿

| ID | リスク | 影響 | 緩和・確認タスク |
|---|---|---|---|
| R-01 | 公式Runner更新でsupervisorが壊れる | 実行不能・重複プロセス | LF-003/019、TC-022 |
| R-02 | WSLが意図せず停止・Guestが孤立 | Linux job停止 | LF-004、TC-005/014/035 |
| R-03 | App認証/refresh/権限が想定と異なる | 初期導入不能 | LF-006/012、TC-020 |
| R-04 | jobがホストユーザーの秘密へ到達 | PC・アカウント侵害 | LF-020、信頼対象限定・隔離の限界明示 |
| R-05 | selector判定直後に停止・競合 | job待機・hosted手動復旧 | LF-014、TC-011、事前選択の表示 |
| R-06 | 登録中断でremoteだけ作成される | Runner重複・孤児 | LF-007/008/018、TC-021 |
| R-07 | 誤対象削除・パス経由の巻込み | 利用者データ消失 | LF-018/020、TC-015/018/031 |
| R-08 | WSL global設定が他作業に影響 | 開発環境停止 | LF-025、TC-019、MVPで自動変更しない |
| R-09 | ログやUIでtoken漏洩 | 権限悪用 | LF-012/013/020、TC-023/040 |
| R-10 | Agent/UI/Guest/DBの更新不整合 | 起動不能・データ破損 | LF-019、TC-027/030 |
| R-11 | hosted利用許可・費用の認識違い | 想定外の請求 | LF-014/015、課金許可とselector利用分の明示 [S20](19_SOURCES.md#s20) |
| R-12 | 機能追加でMVPが成立しない | 開発停滞 | LF-022、P1/P2の境界とゲートを維持 |

## 4. 開発開始前後に決める事項

| 項目 | 既定の扱い | 決定する時点 |
|---|---|---|
| 製品名・repo名 | Runner Dock(2026-09-20決定)。repoは`Runner_Dock` | 商標・既存名の確認は公開前 |
| 永続する内部識別子(label、path、pipe名、credential key) | `runnerdock`を決定案とする(ADR-015) | LF-007/008の実Runner登録前(ADR-015) |
| jobと管理資格情報・管理IPCの境界 | ADR-014の決定案 | LF-005/006/012の検証後、G4で確定 |
| 公開ライセンス | 未決定 | ソース公開前 |
| GitHub App登録所有者 | 未作成・未決定 | LF-006の認証PoC前 |
| Windowsテスト機 | Windows 11 x64想定 | LF-003前に実構成を記録 |
| WSL distro | Ubuntu 24.04を試験基準 | LF-004前にexisting/専用を選択 |
| 署名・更新metadata配布先 | 未契約・未作成 | LF-019、公開前 |
| テレメトリー | なし | 導入する場合は独立した同意・設計 |
| 厳密な同時job上限 | MVP保証しない | LF-025または別スケジューラ設計時 |
| 認証broker | なし | LF-027が必要となった場合のみ |

これらはMock-first開発を妨げません。名称や公開先の未決定を理由にLF-001/002を止める必要はありません。一方、署名・鍵・外部アカウントを開発者やAIが無断で作成しないでください。
