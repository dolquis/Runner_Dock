# 05. ドメインモデルと状態遷移

**区分:** 規範 / **目的:** ローカル状態、GitHubの観測、利用者の意図を混同しない

## 1. エンティティ

| 型 | 主な項目 | 不変条件 |
|---|---|---|
| Node | id、表示名、Windows SID、host情報 | MVPでは物理PC 1台と所有ユーザー文脈の組 |
| Backend | id、node_id、kind、distro識別情報、能力 | `NativeWindows`または具体的なWSL2環境 |
| Scope | id、GitHub host、repo/org、remote ID | 名前変更と認証失敗を区別できる |
| Runner | id、backend_id、scope_id、remote_runner_id、labels、path | 登録先scopeは1つ。複数repo用の単一個人Runnerと誤認しない |
| CredentialRef | id、用途、権限要約、expires_at | 秘密本体を含まない |
| Operation | id、request_id、対象、kind、phase、結果 | 同一request_idを再受信しても重複副作用を起こさない |
| Observation | value、observed_at、verified_at、source、freshness | 失敗や古い観測を現在の事実として扱わない |
| RoutingPolicy | trusted refs、allow_hosted、対象label、revision | ポリシーと生成Workflowの対応を追跡可能 |
| ManagedResource | 種別、識別子、owner、作成operation、状態 | 所有権の確認なしに破壊しない |

repo-level Runnerは1つのrepoに登録されます。複数repoを扱う場合は別のRunner登録かOrganization-levelの共有を設計します。[S21](19_SOURCES.md#s21)

## 2. 希望状態

`DesiredState = Running | Stopped | Removed`

希望状態と進行操作は別です。停止要求を出しても、Busyのため未停止なら`desired=Stopped`、`operation=WaitingForIdle`、`local=Running`を同時に表示します。

## 3. 観測状態

```text
LocalRuntime:
  NotProvisioned / Starting / Running / Stopping / Stopped / Lost / Error / Unknown

RemotePresence:
  Unchecked / Registered / NotFound / Unknown

RemoteAvailability:
  OnlineIdle / OnlineBusy / Offline / Unknown

ObservationFreshness:
  Fresh / Stale / NeverObserved
```

`Registered`と`Online`は別概念です。ローカルプロセスが存在しても、GitHubが利用可能と認識しているとは限りません。REST APIからの`status`と`busy`は型を確認して解釈します。[S02](19_SOURCES.md#s02)

## 4. UIへ出す有効状態

| 条件 | 有効状態 | 表示上の注意 |
|---|---|---|
| Local Running、fresh OnlineIdle | Ready | 稼働しており受付可能という観測。予約済みではない |
| fresh OnlineBusy | Busy | Local Lostなら矛盾警告を併記する |
| Local Running、fresh Offline | Disconnected | ネットワーク・Runner更新・接続の確認を案内 |
| Local Stopped、fresh OnlineIdle | Reconciling | GitHub反映待ちかPID把握不足。緑にしない |
| API認証失効、Local Running | MonitoringUnavailable | Runnerが落ちたと断定しない |
| 観測なし、stale、未知enum | Unknown | 最後の既知値を副表示する |
| Windows Ready、WSL Error | Partial | Node全体を成功にしない |

鮮度既定値はLocal 10秒、Remote 90秒とします。これは製品の設計値でありGitHubのSLAではありません。304で状態が不変と確認できた場合は`verified_at`を更新できます。

## 5. 起動操作の遷移

```text
Requested → Preflight → Preparing → Registering → Starting → Verifying → Succeeded
                │            │            │           │          │
                └────────────┴────────────┴───────────┴──────────┴→ Failed/Partial
```

既に準備・登録済みなら該当段階をスキップします。認証に失敗しただけでRunnerを作り直しません。登録のHTTP応答後にローカル保存へ失敗した場合は、operation IDを含む管理名/label、保存したremote ID、GitHub照会で再照合します。

Windows成功・WSL失敗の場合は、成功側を自動削除しません。起動継続と片側のみ再試行の選択肢を出します。

## 6. 停止操作の遷移

```text
Requested → FreshHealthCheck
  ├─ Busy / Unknown → WaitingForIdle または RequiresForceConfirmation
  └─ Idle           → StopSignal → VerifyExit → Succeeded
```

`WaitingForIdle`は厳密なdrainではありません。受付を原子的に止める仕組みがない限り、Idle観測から停止までに新しいジョブが割り当てられる競合が残ります。画面に「次ジョブを絶対に受け付けない」と表示しません。

強制停止は別コマンドとし、確認ダイアログに対象Runner・観測時刻・中断の可能性を表示します。APIが不明な場合も確認なしの停止へ進めません。

## 7. Agent再起動時

DBにある古いLocal状態は`Unknown`として開始します。WindowsプロセスはPIDに加えて開始時刻、管理対象パス、所有SID、管理識別子を照合します。WSL Guestはconnection generationとセッションnonceで照合します。

OS側の同定に失敗したプロセスを「自分のRunner」としてkillしません。孤児候補として表示し、ユーザー確認または安全な再照合が必要です。

## 8. 同時実行の意味

MVPの`max_active_runners`は起動中のRunner Listener数の上限です。Windows/Linuxを同時起動する既定プロファイルは2です。これはビルドのCPUスレッド数やリポジトリ横断のGitHubジョブ数制御ではありません。

「2つのRunnerを常時onlineに保ちながら、全repoで同時ジョブを必ず1つにする」という機能はMVPには含めません。1に設定する排他プロファイルでは、どちらのBackendを有効にするかを選択し、選択外は停止します。

## 9. コア不変条件

- Runnerのremote identityは`(GitHub host, scope type, scope ID, runner ID)`で識別する。
- Runner名は表示用。曖昧な部分一致で操作対象を選ばない。
- `Unknown`は`false`ではない。Busy不明もIdleにはならない。
- Agentのみがdesired stateを更新する。
- 管理対象外のリソースへ停止・削除・更新しない。
- UI通知成功と処理完了成功を区別する。
- `Removed`への到達にはローカルとremote登録の扱いを記録する。片側だけ削除できた場合はtombstoneを残す。
