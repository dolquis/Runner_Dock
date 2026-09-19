# 06. GitHub認証・API連携仕様

**区分:** 外部仕様＋実装方針 / **基準API版:** `2026-03-10` [S03](19_SOURCES.md#s03)

## 1. 認証方式の決定

初期製品の主経路は**GitHub AppのDevice Flow**、開発時・代替経路はfine-grained PATとします。GitHub Appそのものの登録、配布者の管理、必要権限、Device Flow有効化は製品側の準備事項です。文書パッケージはAppを作成していません。

Device Flowは初回認証だけでなくトークン更新までLF-006で検証します。確認した公式資料では、GitHub Appのrefreshでdevice flow由来のtokenは`client_secret`必須条件から除かれています。[S08](19_SOURCES.md#s08)[S09](19_SOURCES.md#s09)

以前の「GitHub App + Authorization Code + PKCEならデスクトップだけでよい」という前提は採用しません。PKCEと`client_secret`の要件は別です。認可コードフローを採用する場合は、秘密を持つ認証ブローカー等の追加設計を先に行います。OAuth Appのloopback対応を、そのままGitHub Appの仕様として引用しません。[S08](19_SOURCES.md#s08)[S10](19_SOURCES.md#s10)

## 2. Device Flowの流れ

```text
Agent → GitHub: device code要求
UI ← Agent: verification URI / user code / 有効期限だけ
利用者 → 通常ブラウザ: 内容を確認して承認
Agent → GitHub: 指示されたintervalでpoll
Agent: access/refresh tokenを資格情報ストアへ保存
Agent → GitHub: 対象repoと必要操作の権限を確認
```

`authorization_pending`、`slow_down`、拒否、期限切れ、ネットワーク失敗を分けます。UIを閉じたときに認証操作を継続するか取消するかを明示し、認証取消で既存Runnerを停止しません。

User tokenの既定期限やrefresh tokenの期限はレスポンスを優先します。現在の公式説明はaccess 8時間、refresh 6か月ですが、これらを無条件の定数として有効判定に使いません。[S08](19_SOURCES.md#s08)

## 3. 資格情報を3用途に分ける

| 用途 | 保存先・扱い | Runnerへの継承 |
|---|---|---|
| 登録/削除などの管理認証 | Windows側のcredential adapter。権限・期限を表示 | 禁止 |
| 登録用短命token | 登録Operationのメモリのみ。終了後破棄 | 設定処理へ限定的に渡す |
| Workflow selector用読取token | GitHub Actions Secretとしてユーザーが別途設定 | ビルドjobへ渡さない |

管理Appのuser tokenを、そのままselector用Secretへ自動複製しません。ローカルPCが止まっている間もselectorは動く必要があり、ローカル側のrefresh処理には依存できません。

MVPのselectorには対象repo限定・Administration readのfine-grained PATを利用者が設定する手順を提供します。期限、更新方法、削除方法を表示します。登録・削除用のwrite tokenを入れさせません。

## 4. 必要権限

| 操作 | repo-level | org-level |
|---|---|---|
| Runner一覧・個別取得・配布物一覧 | Administration: read | Self-hosted runners: read |
| 登録token発行・解除token・登録削除 | Administration: write | Self-hosted runners: write |
| Workflowファイル閲覧 | Contents: read | 対象repo側のContents権限 |
| Workflow書換え | MVP対象外。後続でContents/Workflows等をendpoint別確認 | 同左 |
| ジョブ詳細の取得 | 後続のActions: read機能として分離 | 対象repo側のActions権限 |

Runner管理権限は広い影響を持つため、「ログインするだけの軽い権限」と説明しません。App/tokenに権限があっても、認証ユーザー・App installation・組織ポリシーが対象にアクセスできなければ失敗します。[S02](19_SOURCES.md#s02)[S08](19_SOURCES.md#s08)

通常の`GITHUB_TOKEN`だけでRunner管理APIが使えると仮定しません。足りない権限をWorkflowの`permissions`に存在しないキーを書いて補いません。

## 5. 使用endpoint

```text
GET    /repos/{owner}/{repo}/actions/runners
GET    /repos/{owner}/{repo}/actions/runners/{runner_id}
GET    /repos/{owner}/{repo}/actions/runners/downloads
POST   /repos/{owner}/{repo}/actions/runners/registration-token
POST   /repos/{owner}/{repo}/actions/runners/remove-token
DELETE /repos/{owner}/{repo}/actions/runners/{runner_id}
```

Organization対応では`/orgs/{org}/actions/...`へ明示的に切り替えます。repo用pathでorg Runnerが全件取れるという前提は置きません。登録用tokenは短命であり、Runnerの実行資格情報とは別です。[S02](19_SOURCES.md#s02)

## 6. API clientのルール

`Accept: application/vnd.github+json`、明示API版、製品User-Agentを付けます。API hostを設定から任意URLへ自由変更する機能はMVPに入れません。

一覧は`Link`の`next`をたどり、1ページ目だけで存在しないと判定しません。次URLのoriginを検証し、別hostへAuthorizationを転送しません。redirectはAPI originと認証ヘッダーの扱いを確認して追従します。[S23](19_SOURCES.md#s23)[S24](19_SOURCES.md#s24)

読取はETagを利用可能なら利用し、アカウント単位で重複要求をまとめます。既定pollは30秒、GUI非表示では60秒を初期値とし、起動直後だけ短い限定再確認を許可します。`Retry-After`、残量、reset、poll intervalが返れば優先します。[S23](19_SOURCES.md#s23)

## 7. 障害の扱い

| 状況 | 内部エラー | UI / リカバリー |
|---|---|---|
| 401 | AUTH_EXPIRED | 再認証。Runner自体は停止しない |
| 403 | PERMISSION_OR_POLICY | 必要権限/組織承認/rate limitを切り分ける |
| 404 | RESOURCE_OR_PERMISSION_UNKNOWN | 権限不足による隠蔽の可能性を考慮。即再登録しない |
| 429またはrate limit | RATE_LIMITED | 指示された時刻まで停止、観測をstaleにする |
| 5xx/通信断 | REMOTE_UNAVAILABLE | 指数backoff＋jitter。最終成功時刻を保持 |
| 未知のstatus/不正busy型 | RESPONSE_UNSUPPORTED | Unknownとして記録。推測でIdleにしない |
| 部分的なページ取得 | INCOMPLETE_RESULT | 不在を確定しない |

POSTは応答を失った可能性があるため単純に繰返しません。操作内容に応じて結果を照合してから再試行します。Runnerの新規登録は一意の管理識別子とOperation記録を使います。

## 8. トークン失効・削除

ログアウトは資格情報を削除しますが、既存Runnerのremote登録やRunner固有の資格情報まで自動消去するとは限りません。ログアウト画面に「監視認証の削除」と「Runner停止・登録解除」を別操作として表示します。

実装ではsecret型のDebug表示を抑止し、URL query、ログ、crash report、clipboardへアクセストークンを出しません。登録tokenを公式configに渡す方式は実配布物で検証します。CLI引数が必要な場合は短命登録tokenに限定し、プロセス一覧からの露出を残存リスクとして記録します。PATを引数へ置くことは禁止です。
