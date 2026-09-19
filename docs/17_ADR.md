# 17. Architecture Decision Records

**状態表記:** Accepted=初期方針、Proposed=実装前検証が必要、Deferred=MVP後。Acceptedも実機で証明済みという意味ではありません。

## ADR-001: 独自開発と参考実装の境界

**状態:** Accepted / **背景:** ユーザーはHomeRunのソフトフォークを希望していない。

**決定:** git履歴・ソース・UIアセットを移植せず、独自の要件、ドメイン、プロトコル、実装を作る。HomeRunは問題の分解、運用UX、失敗箇所の理解に参照する。

**比較:** Forkは既存資産を利用できるが、今回の開発方針に反する。すべての先行知見を避ける必要もない。

**帰結:** 作成物の由来を記録する。将来コードを取り込む提案が出たら、ライセンス条件とユーザー方針を別途確認する。独自開発だから第三者ライセンスへの確認が不要とは扱わない。

## ADR-002: Node→Backend→Runnerを中心にする

**状態:** Accepted

**決定:** 物理PCをNode、実行環境をBackend、その環境に登録するGitHub実行主体をRunnerとする。GitHub上のscopeとRunner IDはローカルIDから分離する。

**比較:** Runner一覧だけでも起動停止は可能だが、WSLや物理資源の共有・一括管理を表しにくい。

**帰結:** Nodeの総合状態とRunner個別状態を両方表示する。MVPの対象が一台一repoでもモデルをrepo名文字列に埋め込まない。

## ADR-003: GUIと非昇格user Agentを分離する

**状態:** Accepted / **検証:** LF-003/004/005

**決定:** GUIは薄いクライアント。ログオンユーザーのAgentが希望状態・実行操作・永続化を所有する。新規RunnerのsupervisorもAgentに統一する。

**比較:** GUI内だけで管理すると閉じた時の寿命が曖昧になる。常時昇格サービスはWSLのユーザー文脈と権限管理を複雑にする。

**帰結:** GUI終了後も継続できるが、ログアウト後継続はMVP対象外。将来の専用アカウント/サービスモードは別ADR。既存service管理を混ぜない。

## ADR-004: WSLを長寿命Guestで管理する

**状態:** Proposed / **採用ゲート:** G2

**決定:** `wsl.exe`経由で非root Guestを起動し、stdioを専用protocolへ使う。Guestが公式Linux Runnerを管理する。

**比較:** `wsl.exe ... /bin/true`は単なる起動要求であり、systemdサービスだけでもWSLの継続稼働を保証しない。[S04](19_SOURCES.md#s04) SSHサーバー導入は不要な設定・到達面を増やす。

**帰結:** Guest寿命、EOF、Windows側死亡、sleep/resumeを実機検証する。systemdは将来の既存service取込みに残すが、MVP新規Runnerの常駐条件にしない。

## ADR-005: GitHub公式Runnerと公式wrapperを使う

**状態:** Accepted / **検証:** G1/G2

**決定:** GitHub Actions通信部分は公式Runnerを利用し、公式`config`/`run`wrapperを尊重する。独自Runnerプロトコルは実装しない。[S01](19_SOURCES.md#s01)[S21](19_SOURCES.md#s21)

**比較:** Listener直接起動は簡単に見えても、更新・再起動・終了コードの契約を外す可能性がある。公式の実装詳細は検証して追従する。

**帰結:** 展開検証、更新監視、バージョン表示を本体更新と分ける。未知の終了パターンを無限再起動しない。

## ADR-006: ローカル認証はGitHub App Device Flowを第一候補にする

**状態:** Proposed / **採用ゲート:** G3

**決定:** 公開デスクトップアプリに共有client secretを埋め込まない。Device Flowとそのrefreshを実証する。開発用PATを補助として残す。[S08](19_SOURCES.md#s08)[S09](19_SOURCES.md#s09)

**比較:** GitHub Appの認可コード+PKCEは、公式のcode交換でclient_secretも要求される。OAuth Appのloopback案内をそのままGitHub Appへ移せない。[S08](19_SOURCES.md#s08)[S10](19_SOURCES.md#s10)

**帰結:** 認証brokerなしのMVPを目指すが、認証・refresh・権限のゲートが通らない場合はADRを再検討する。通常token更新のためだけに秘密鍵を配布しない。

## ADR-007: ローカル管理tokenとWorkflow監視tokenを分離する

**状態:** Accepted

**決定:** GUI/Agent用権限は登録等に必要なもの、Workflow側は対象Runnerの読取だけ。MVPのWorkflow監視は別のread-only PATを利用者がSecretとして設定する。[S02](19_SOURCES.md#s02)

**比較:** ローカルで更新される短期tokenをworkflowへコピーすると、自宅PCが停止中の独立動作と期限管理に問題がある。GitHub App秘密鍵を全利用者へ共有する方式は採用しない。

**帰結:** セットアップに一手間は残る。期限・費用・権限をUIで説明する。将来brokerを導入するなら可用性と運用責任も設計する。

## ADR-008: フォールバックは事前選択と明示する

**状態:** Accepted

**決定:** 信頼条件・課金許可・remote online/idle/labelを確認したうえで後続jobのruns-onを選ぶ。選択は予約ではない。[S01](19_SOURCES.md#s01)[S02](19_SOURCES.md#s02)[S22](19_SOURCES.md#s22)

**比較:** ローカル失敗後に無条件で同jobを再実行すると、副作用や二重deployが発生し得る。GitHubのlabel配列を優先順位指定として使うこともできない。

**帰結:** 「完全自動フェイルオーバー」は製品文言にしない。待機後の自動差し替えはMVPに入れず、新規force_hosted実行を案内する。

## ADR-009: 既存WorkflowはMVPで自動書換えしない

**状態:** Accepted

**決定:** 固定labelの単純jobを説明し、新規テンプレートを生成する。未知構文・複雑matrix等は対応外として表示する。

**比較:** 正規表現によるYAML書換えは、式、権限、needs、コメントを壊しやすい。GitHubへの直接commitは更に権限と副作用を増やす。

**帰結:** 後続パッチはCST/AST、diff確認、読取hash検証を必須にする。ユーザーが変更内容を確認できることを優先する。

## ADR-010: 秘密と特権をUIから分離する

**状態:** Accepted / **検証:** G4

**決定:** 秘密はWindows Credential adapterで管理し、SQLiteには参照だけを保存。Tauri capabilityとnamed pipeの権限を最小化する。[S11](19_SOURCES.md#s11)[S26](19_SOURCES.md#s26)[S27](19_SOURCES.md#s27)

**比較:** すべてを暗号化JSONへ保存する方式は鍵管理を別に解く必要がある。UIへshellや任意ファイルアクセスを渡す方式は制御境界が広すぎる。

**帰結:** 手入力PATの一時的取扱いもレビューする。同一OSユーザーで動く悪意あるjobから秘密を完全隔離できるとは宣言しない。[S19](19_SOURCES.md#s19)

## ADR-011: 現行stableを検証して固定する

**状態:** Accepted / **候補:** [技術選定](04_TECHNOLOGY_STACK.md)

**決定:** Tauri 2、Rust、React/TypeScript、Vite、Tokio、SQLiteを採用候補とし、LF-001で互換検証後にlockする。

**比較:** nightly/RCや浮動latestは初期の再現性を下げる。以前の会話で挙げた具体的な版を無検証で固定することもしない。

**帰結:** 更新は独立PR。依存の最新値とプロジェクト採用値を分けて記録する。技術の新しさを、検証省略の理由にしない。

## ADR-012: 同時接続数と同時job数を混同しない

**状態:** Accepted

**決定:** Windows/WSLを二つとも待受けにすれば二つのjobが走り得る。MVPは利用可能listener数とメトリクスを提供し、PC全体の厳密なjob数上限を保証しない。

**比較:** Runnerのbusy照会やGitHub concurrencyだけで、別repo・別workflowを含むホスト全体の排他制御が完成したとは扱えない。

**帰結:**厳密な予約/排他/資源制限は別要件と検証を必要とする。`.wslconfig`はWSL2全体に影響し、特定Runnerだけの設定と説明しない。[S05](19_SOURCES.md#s05)

## ADR-013: Sandbox、Docker、ephemeralは後続に分離する

**状態:** Deferred

**決定:** persistent Runnerの安全な運用支援をMVPとし、ephemeral登録・環境再作成・ホスト隔離は別機能とする。[S01](19_SOURCES.md#s01)[S07](19_SOURCES.md#s07)[S19](19_SOURCES.md#s19)

**帰結:** ephemeralフラグで一度のjob後に登録が消えても、ホストへの変更が元に戻るとは扱わない。Scale Set Client等を導入する場合も、Go側の別プロセス統合と運用コストを評価する。
