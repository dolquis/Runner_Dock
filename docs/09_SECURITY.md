# 09. セキュリティ設計・脅威モデル

**区分:** 公開MVPの必須基準 / **前提:** self-hostedは任意コードの実行装置

## 1. 保護対象と信頼境界

保護対象はGitHub管理資格情報、Runner資格情報、私用ファイル、SSH鍵、ブラウザ情報、他WSL環境、GitHub repo、配布署名鍵です。境界はWebView→Rust、GUI→Agent、Windows→WSL、管理プロセス→ビルドプロセス、ローカル→GitHub、更新サーバー→実行ファイルです。

GitHubはpublic repoでのself-hosted利用に強い注意を示しています。private repoであっても、誰が実行コードや依存関係を書き換えられるかが重要です。[S19](19_SOURCES.md#s19)[S21](19_SOURCES.md#s21)

## 2. 保証しないこと

同じWindowsユーザーで動くジョブは、そのユーザーの権限を持ちます。Credential ManagerやDPAPIは秘密の平文保存を避ける手段ですが、同一ユーザーとして実行される敵対的コードからの完全な防壁ではありません。

WSLはWindowsファイルへのアクセスやWindows executableとの相互運用を持ちます。専用distro、automount無効化、interop制限は露出低減策であり、私用PCからの完全隔離を意味しません。[S07](19_SOURCES.md#s07)

したがってMVPは**所有者が信頼するprivate repoのコードのみ**を対象とし、専用Windowsユーザーまたは専用PCを推奨します。任意の外部PRを安全に動かす機能は提供しません。

## 3. 脅威と対策

| ID | 脅威 | 主対策 | 残存リスク |
|---|---|---|---|
| TH-01 | PR/依存関係から私用PC侵害 | localの信頼対象限定、最小権限、専用環境推奨 | trusted mainにも悪意あるコードは入り得る |
| TH-02 | WebViewのXSSからOS操作 | remote content禁止、CSP、限定command、入力検証 | 許可command自体の欠陥 |
| TH-03 | named pipe経由の他ユーザー操作 | 明示DACL、SID照合、remote client拒否 | 同一SIDの敵対的プロセスは別問題 |
| TH-04 | tokenのログ/環境変数漏洩 | secret型、マスキング、継承環境のallowlist | process memoryや登録時短命argv露出 |
| TH-05 | shell injection | 固定実行file、引数生成検証、任意shell APIなし | Windowsのquotingに実機検証が必要 |
| TH-06 | 誤ったサービス/distro削除 | immutable ID、所有権台帳、plan→confirm→apply | 外部からの名前・path変更 |
| TH-07 | 不正更新/供給網侵害 | 署名、checksum、lock、SBOM、review | 正規鍵/依存元が侵害される場合 |
| TH-08 | Busy中の停止・自己更新 | fresh check、待機、強制操作の確認 | 割当競合による新ジョブ中断 |
| TH-09 | 再試行で重複登録/二重ビルド | idempotency、照合、明示再実行 | ネットワーク分断で結果が曖昧な区間 |
| TH-10 | 料金を使う意図しないfallback | hosted同意、policyと理由表示 | GitHub側の同時実行・他workflow利用 |
| TH-11 | ログ洪水・巨大payload | バッファ/サイズ/保存上限、drop表示 | ログ欠落はあり得る |
| TH-12 | Runner自身が制御ファイル改変 | 分離path、検証、専用ユーザー、破壊操作再確認 | 同一ユーザー実行は完全分離ではない |

## 4. 権限設計

GUI、Agent、通常Runnerは非昇格です。管理者権限を要求する初期処理は、変更対象・引数・nonceを固定した短命処理にします。書換え可能なユーザーディレクトリの任意スクリプトを「最上位権限の定期タスク」へ登録しません。

初期MVPの起動タスクは通常ユーザーのログオン時起動です。Task SchedulerをUAC回避の汎用ブローカーとして公開しません。

LinuxでもRunnerをrootで起動しません。必要パッケージの導入や専用ユーザーの作成はbootstrapとして区別し、日常運用に無制限`sudo`を要求しません。

## 5. TauriとIPC

本番WebViewは同梱frontendだけを表示し、外部リンクはシステムブラウザへ渡します。HTTP、shell、filesystem、SQL、credential storeの汎用権限をfrontendへ付与しません。必要なTauri capabilityは用途・windowを限定してレビューします。[S11](19_SOURCES.md#s11)

named pipeの既定ACLに依存せず、AgentのSIDと必要な管理主体だけへ権限を与えます。要求ごとにpayload型、長さ、対象ID、操作権限を検証します。pipe名を秘密情報として使いません。[S26](19_SOURCES.md#s26)

pipe名は予測できるため、名前が一致したことを相手の同一性として扱いません。TH-03のSID照合は双方向に適用し、待受側はDACLで接続元を限定し、接続側は待受けているpipeの所有SIDを自分のSIDと突き合わせてから要求を送ります。不一致の場合はAgentを起動せずに拒否します。

同一SIDからのアクセスを許すことは、Runnerジョブから完全に隠すことではありません。jobから管理IPCと管理資格情報へ届く経路を減らす方針は[ADR-014](17_ADR.md)で検証します。管理権限を持つ別ユーザー/別サービスへ分離する将来設計は、アクセス制御を改めて検証します。

## 6. Secretの取り扱い

長寿命token、refresh token、PATはWindows credential adapterに保存し、SQLiteには参照IDだけを置きます。frontendへ返す通常DTOに秘密を含めません。token入力後は入力欄をクリアし、telemetry、crash report、clipboard履歴への露出を抑えます。[S27](19_SOURCES.md#s27)

Runner実行環境には`GH_TOKEN`、管理用`GITHUB_TOKEN`、App秘密鍵を継承させません。Windows/Linuxの機能に必要な環境変数と、利用者が承認したtoolchain設定のみ明示的に渡します。

Runnerの`.credentials`等は公式Runnerの管理対象です。diagnostic export、バックアップ、ログbundleから除外します。workspace・キャッシュ・artifactにも秘密が残り得るため、ログだけを消せば安全とは説明しません。

## 7. Workflowの安全性

selectorはcheckoutしないhosted jobとして生成します。read-only監視Secretはtrusted event時のselector stepだけに渡し、build jobへ持ち込みません。

外部/内部PR、`pull_request_target`、`workflow_run`での未信頼artifact利用、dynamic ref、任意pathのscript実行は分析対象です。MVPは判断不能なWorkflowを自動書換えしません。

public repoの標準hostedは無料であるという確認時点の料金体系も、危険なself-hosted導入を避ける判断材料として提示できます。[S20](19_SOURCES.md#s20)

## 8. 削除・更新・復旧の境界

通常停止は対象Runnerだけ、distro終了は別操作、distro削除はさらに別操作です。アンインストールが自宅の開発Ubuntuや他のRunnerを削除してはいけません。

Updater署名とWindows Authenticode署名は別の役割です。秘密鍵をrepo、Runner workspace、公開artifactへ置きません。テスト用WebDriver/IPC mocking機能を本番バイナリへ含めません。[S13](19_SOURCES.md#s13)[S14](19_SOURCES.md#s14)

## 9. 公開前のセキュリティゲート

TH-01〜TH-12に対して、対策テストまたは残存リスクの利用者への説明が必要です。重大な既知の秘密漏洩、権限昇格、任意path削除は未解決で出荷しません。

診断ZIPへfake secretを混ぜた試験を行い、既知パターンに限らずHTTP header、環境、argv、Runner設定ファイルが含まれないことを確認します。完全自動マスキングを保証せず、書出し前プレビューも用意します。
