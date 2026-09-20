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

## ADR-014: jobから管理資格情報・管理IPCへの到達を減らす

**状態:** Proposed / **採用ゲート:** G4 / **検証:** LF-005/006/012

**背景:** ADR-003によりAgentとRunnerは同じWindowsユーザーで動く。GitHub Appのuser tokenはAppの権限を引き継ぐため、常時監視に使うtokenもAdministration: writeを持つ。信頼済みrefに混入した依存コードは、同一ユーザーとして資格情報ストアを読み、同一SIDのnamed pipeへ管理要求を送り得る。[S19](19_SOURCES.md#s19)[S27](19_SOURCES.md#s27)

**決定案:** MVPでは次を組み合わせる。(1) 管理IPCの破壊的method(`runner.applyRemove`、`node.forceStop`、`credential.import`、`settings.apply`)には対話確認challengeを必須にする。ただしpipeの認可は同一SIDに基づくため、同じユーザーで動くjobはGUIと同じchallenge手順を完了できる。したがって(1)は**誤操作と単純な自動化に対する保護**であり、敵対的な同一ユーザーコードに対する境界とは扱わない。補強として、Agentはpipe clientのプロセスIDから実行fileのpathと署名を検証し、製品のGUI/CLI以外からの破壊的methodを拒否する。これも同一ユーザーによるプロセス注入で回避され得る多層防御の一層である。(2) write権限を要する操作の直前にだけ再認証またはtoken取得を行い、長期保存するのはrefresh不要で足りる最小の資格情報に限る方式をLF-006/012で検証する。(3) 常時監視をread専用資格情報へ分離できるかを同タスクで確認する。

**比較:** Runnerを別のWindowsユーザーで動かす方式は境界として最も明確だが、初回に管理者権限でのユーザー作成、WSLのユーザー文脈、プロファイル分離が必要になり、ADR-003の非昇格方針と衝突する。MVP後の「専用アカウントモード」として別ADRで扱う。何も対策せず注意書きだけにする案は、管理tokenの影響範囲(repo管理権限)に対して弱い。

**帰結:** 敵対的な同一ユーザーコードへの実効的な低減策は(2)(3)、すなわち奪われ得る権限と常駐時間を減らすことであり、(1)はそれを代替しない。(1)〜(3)でも完全な防御にはならない。残存リスクを[09](09_SECURITY.md)のTH-12と利用上の注意に記載する。検証の結果(2)(3)が成立しない場合は、write tokenを常駐させる前提でリスク説明を強め、専用アカウントモードの優先度を上げる。

## ADR-015: 永続する識別子を表示名から分離する

**状態:** Proposed / **決定期限:** LF-007/008で実Runnerを登録する前

**背景:** 製品名はRunner Dockに決定したが、商標確認等で表示名が変わる可能性は残る。一方、custom label、Runner名prefix、`%LOCALAPPDATA%`配下のdirectory、pipe名、WSL内path、資格情報ストアのkeyは、実Runnerの登録後にGitHub側・利用者のWorkflow・ローカル環境へ残り、改名時に移行が必要になる。

**決定案:** 表示名とは独立した内部識別子を`runnerdock`(小文字・区切りなし。Windowsのdirectory名は`RunnerDock`)と定め、上記すべての永続名はその識別子から導出する。表示名・window title・文書上の名称だけを後から変更可能とする。識別子は定数1か所に集約する。

**帰結:** 内部識別子は公開名称の確定を待たずに決められる。既存名との衝突確認は識別子についても行う。識別子を変更する場合はlabel・path・credential keyの移行手順を伴う。

## ADR-016: 公開ライセンスをMIT OR Apache-2.0とする

**状態:** Accepted / **決定日:** 2026-09-20(所有者の決定)

**決定:** 文書とソースコードを、利用者がMITまたはApache-2.0を選択できるデュアルライセンスで公開する。著作権表示は`Copyright (c) 2026 dolquis`。Rust crateには`license = "MIT OR Apache-2.0"`を記載する。

**比較:** MIT単独は簡潔だが特許許諾の明文がない。Apache-2.0単独は特許条項を持つがGPLv2との組合せに制約がある。GPL系は改変物の公開を求められるが、所有者にクローズドな有料化の予定はなく、採用障壁の低さを優先した。採用予定の主要依存(Tauri、Tokio、SQLx、windows-rs)はMITまたはApache-2.0系である。

**帰結:** 第三者による商用・クローズドな利用を許容する。公開済みの版に付与した許諾は撤回できない。製品名とロゴの利用権はライセンスに含めない。依存関係のライセンス適合とNOTICEの要否はLF-022で確認する。HomeRunのコードは取り込まないため、同プロジェクトのMIT表示を引き継ぐ必要はない(ADR-001)。

## ADR-017: ワイヤー契約の生成を`crates/protocol`側の薄いツールに閉じる

**状態:** Accepted / **決定日:** 2026-09-20(LF-002で実装)

**背景:** [技術選定](04_TECHNOLOGY_STACK.md)§4は、`crates/protocol`のserde DTOからJSON SchemaとTypeScriptを生成する方針を示しつつ、crateのstable状況とnullable/enum/u64表現の確認をLF-002へ委ねていた。Rust側の型生成crateには、通信契約をツール固有の出力形式へ縛るもの、生成をtest実行に結び付けるもの、`u64`をJavaScriptのnumberへ落とすものがある。

**決定:** JSON Schemaの生成にschemarsを使い、TypeScriptはschemarsの出力から`crates/protocol`内の`contracts-gen`が直接組み立てる。生成物は`packages/contracts/`へcommitし、同じバイナリの`--check`が再生成結果と突き合わせてCIを止める。`pnpm contracts:check`はこのコマンドを包む。

**比較:** TypeScript生成crateを追加する案は記述量が減るが、ワイヤー契約の表現がそのcrateの都合に従属する。生成物を持たずビルド時に作る案は差分検出ができない。自前の変換器は、この repo が実際に出すschemaの形(object、文字列enum、`$ref`、配列、`Option`のanyOf、内部タグ付きenum)だけを扱えばよく、想定外の形は`unknown`として表面化する。

**帰結:** `u64`は[DecimalU64](10_IPC_DATA_MODEL.md)として10進文字列で渡り、TypeScript側は`string`になる。`Option`は`| null`、enumは文字列のunion、内部タグ付きenumは判別用`kind`を持つ交差型として出る。生成物を手で編集しない。protocolのDTOを変えたら同じPRで再生成する。schemaの新しい形を使い始めるときは変換器の対応を先に足す。
