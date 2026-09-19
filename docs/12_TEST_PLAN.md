# 12. テスト戦略・受入試験

**区分:** 実装・公開ゲート / **現状:** 製品コード未作成のため、以下の製品試験は未実行

## 1. 検証階層

| 層 | 対象 | 実行場所 |
|---|---|---|
| 単体 | 状態導出、信頼policy、idempotency、DTO、parser | Linux/Windows CI、外部副作用なし |
| 契約 | HTTP fixture、IPC frame、Guest、DB migration | Mockを使うCI |
| UI | 操作・部分成功・Unknown・アクセシビリティ | Vitest/ブラウザMock |
| Native E2E | Tauri、named pipe、Agent再接続 | Windowsテスト環境 |
| WSL統合 | Guest寿命、Linux Runner、停止、復旧 | 使い捨てまたは専用WSL実機 |
| GitHub統合 | 登録・job・事前選択・失効 | 承認されたprivateテストrepo |
| 配布 | installer、署名、更新、uninstall | クリーンなWindows検証環境 |

通常のGitHub-hosted WindowsでWSL2仮想化を常に使えるとは仮定しません。WSL統合は専用検証環境で実行し、一般CIが成功したことから代用しません。

Tauri公式が案内するWebdriverIO対応serviceを評価します。テスト専用のembedded WebDriverやIPC mock機能は本番ビルドに入れないことも試験対象です。[S14](19_SOURCES.md#s14)

## 2. 必須試験カタログ

| ID | 条件・操作 | 期待結果 |
|---|---|---|
| TC-001 | GUIを2回起動 | Agentは1件、同じsnapshotへ接続 |
| TC-002 | Runner起動中にGUI終了・再起動 | Agent/Runnerは継続し、正しい状態へ再接続 |
| TC-003 | busy=false/null/missing/文字列、unknown status | falseだけIdle候補。残りはUnknown/対象外 |
| TC-004 | 通常操作を非昇格ユーザーで実行 | UAC不要。管理者Runnerを要求しない |
| TC-005 | WSLアイドルでGUIを閉じ1時間以上待機 | Guestが維持され、Linux Runnerが応答する |
| TC-006 | Local Running、GitHub Offline/Unknown | 状態を別表示、誤った緑表示なし |
| TC-007 | 401/403/404/429/5xx/timeout | 構造化エラー、適切なbackoff、再登録暴発なし |
| TC-008 | 一覧2ページ目に対象、途中失敗 | 取得完了まではNotFoundにしない |
| TC-009 | fork PR、同一repo PR、public、別ref | 既定経路でlocalを選ばない |
| TC-010 | 両Runner Ready/片側Busy/両方停止 | OSごとに正しい事前選択 |
| TC-011 | selector直後にPC停止/他jobが占有 | 残存競合を再現し、無停止fallbackと表示しない |
| TC-012 | Busy中に通常停止 | 保留。強制停止に確認が必要 |
| TC-013 | Remote stale/Unknownで停止 | 確認なしに停止しない |
| TC-014 | Guest通信切断/Agent強制終了 | 方針どおりの停止・猶予・再照合。孤児を無確認killしない |
| TC-015 | 似た名前のRunner/service/distroを追加 | exact ID対象だけ操作 |
| TC-016 | path/labelに空白、日本語、`&`、引用符 | injectionなし、拒否理由が明確 |
| TC-017 | checksum不一致、巨大archive、`../` entry | 展開・実行前に拒否 |
| TC-018 | symlink/junctionで削除先を外へ向ける | 所有範囲外を削除しない |
| TC-019 | `.wslconfig`既存設定と別WSLが存在 | 無断上書き/全体停止なし |
| TC-020 | Device Flow失効/拒否/slow_down/refresh | 正しい待機・更新・再認証、tokenログなし |
| TC-021 | 登録完了直後にDB保存失敗して再試行 | 結果照合し、重複登録を防止 |
| TC-022 | 公式Runnerの自己更新・再起動 | ラッパー挙動を維持、無限再起動なし |
| TC-023 | fake secretを各ログ/HTTP/argvへ注入 | 診断exportに秘密を含めない |
| TC-024 | 資格情報保存/削除/別ユーザー読取 | 想定のOS権限を確認。過大な隔離保証をしない |
| TC-025 | 別SIDからIPC接続、remote pipe接続 | 拒否。許可済みUIは動作 |
| TC-026 | 不正frame、洪水、遅い購読者 | Agentは生存、上限とresyncを適用 |
| TC-027 | migration失敗、WAL、復元、disk full | 整合性維持、復旧手順と原因表示 |
| TC-028 | 生成YAMLの`on`、JSON配列、式を検査 | 有効な構造。secretをbuildへ渡さない |
| TC-029 | 差分プレビュー後に原本を書換え | 適用拒否、再解析。後続パッチ機能の試験 |
| TC-030 | 本体/Guest互換不一致・不正更新署名 | 更新拒否または安全な互換エラー |
| TC-031 | 削除失敗、remote不達、uninstall | tombstone、復旧手順、他distro保護 |
| TC-032 | 一時Runnerの解除だけを実行 | データ残留を確認。環境破棄と同一視しない |
| TC-033 | 同名Runnerがrepo/orgに存在 | scopeとremote IDで識別 |
| TC-034 | 2Listenerを同時onlineにする | 2ジョブが並行し得る説明・メトリクスが一致 |
| TC-035 | スリープ復帰、ネットワーク切替 | 観測の再取得、Unknown→確認済みへ遷移 |
| TC-036 | 古いevent、時計変更、304応答 | generation/sequence/鮮度が正しく機能 |
| TC-037 | 構築取消・UAC拒否・再起動待ち | 変更済み/未変更を残し安全に再開 |
| TC-038 | 150/200%拡大、Tab操作、Reader | 状態と操作にアクセスできる |
| TC-039 | POST応答を失う、HTTP redirect | 無条件再送や別hostへの秘密転送なし |
| TC-040 | 管理token設定後にRunnerプロセス検査 | 子環境に管理用tokenが継承されない |
| TC-041 | Windows/Linux実ジョブをテストrepoで実行 | 実Runner名とOS、結果がGitHubとUIで一致 |
| TC-042 | CLI start/status/stop/doctor | GUIと同じ権限・Operation・結果を使う |
| TC-043 | logon起動OFF/ON、停止希望保存 | 利用者の同意どおりに起動。無断Runner起動なし |
| TC-044 | CPU/メモリ採取不能・ログ洪水 | Unknownとdrop表示、UI応答性維持 |
| TC-045 | 本番artifact内のテスト用driver/秘密を走査 | 不要機能と機密が含まれない |

TC-029、TC-032、TC-033のフル機能試験は対応する後続機能の公開ゲートです。MVPで実装しない機能を、テスト済みとしてチェックしません。

## 3. 重点fixture

GitHub fixtureには200(online/idle)、200(busy)、未登録、labels不足、unknown enum、schema欠損、401、権限による404、429+Retry-After、301別host、ページ欠落を含めます。token値はfakeのみです。

WSL fixtureにはUTF-16出力、NUL、日本語Windows、ディストリビューション名の空白、WSL1、systemdなし、Linuxユーザー不在、他distro稼働、Ubuntu起動失敗を含めます。

Workflow fixtureには固定label、配列、matrix、reusable、anchors、コメント、既存needs、式、pull_request_target、権限、Windows shell差、empty inputを含めます。

## 4. 性能測定

NFRの値は代表実機で測定します。OS/CPU/RAM/WSL版/Runner版/ソフト版/対象repo/ネットワーク条件を記録し、中央値とp95を分けます。アイドル消費はAgent単体、GUI込み、WSL込みを分け、WSL VM全体のメモリをAgent単体の消費と報告しません。

## 5. CIでの安全性

外部PRのCIはGitHub-hostedで実行し、Runner管理・配布署名・本番tokenを渡しません。自宅統合環境へ流すテストは、人間が確認したtrusted refだけです。

このソフト自身をテストするRunnerを同じテストで停止しないよう、制御対象とテスト実行環境を分けます。自己更新/削除は使い捨て環境を使います。

## 6. 証跡フォーマット

```text
Test ID / 実施日 / 実装commit / OS・WSL・Runner版
事前条件 / 操作 / 期待結果 / 実結果
証跡path / 秘密除去済みか / Pass・Fail・Not run・Blocked
不具合ID / 再試験条件
```

文書パッケージに含まれるYAML/SQLの構文検査と、製品実装の動作試験は別です。構文検査に合格しても「Windows実機で起動成功」とは記載しません。
