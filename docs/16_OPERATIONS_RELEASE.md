# 16. 運用・配布・リリース設計

**区分:** 製品実装後の運用要件 / **状態:** 手順は未実装機能を含む

## 1. 通常運用のモデル

インストール・前提導入・登録を済ませた後は、通常ユーザーとして起動し、Nodeの一括開始・状態確認・停止を行います。初期設計はログオン中のuser Agentです。Windows起動前からのsystem serviceや、ログアウト後の継続実行を保証しません。

GUIの「閉じる」はウィンドウを閉じるだけです。「Node停止」はRunner停止を要求します。「アプリ終了」はAgentの終了方針を別に表示します。Busyまたは状態不明なら、実行中jobへの影響を明示して保留を既定にします。

ログオン時自動起動はopt-inとし、管理者への常時昇格を設定しません。OS全体のスリープ設定を勝手に変更せず、スリープするとRunnerが利用できなくなることを表示します。

## 2. ダッシュボードの診断と対処

| 表示・症状 | 確認する内容 | 非破壊の対処 |
|---|---|---|
| Local Running / GitHub Offline | 接続、選択scope、remote ID、公式Runnerログ | 更新時刻とネットワークを確認。即座に再登録しない |
| GitHub Unknown / 401 | token期限・失効 | 再認証。前回Onlineを現在状態として表示しない |
| 403 / 404 | 権限、App installation、repo access、rate limit | 対象と権限を確認。404だけで登録削除と断定しない [S23](19_SOURCES.md#s23) |
| 429 / Rate limited | Retry-After、同時照会 | バックオフし、残り待機を表示 |
| WSLが停止した | Guest/wsl.exe寿命、OS sleep、ユーザーによる停止 | 対象distroだけを確認。全WSLをshutdownしない |
| Runner重複の疑い | operation journal、GitHub exact ID | 一覧を照合して再接続。名前の先頭一致で削除しない |
| 部分失敗 | Windows/Ubuntu個別Operation | 成功済み対象を壊さず、失敗分だけ再試行 |
| Jobがself-hosted待ち | 事前選択後の停止・競合 | 元実行を確認し取消し、新規のforce_hosted実行を検討 |
| UIとAgentの版が不一致 | protocol major、upgrade状態 | 書込を止め、互換版へ更新。DBを推測で修復しない |

## 3. ログと診断パッケージ

提案初期値はAgentログ10MB×5世代、Runner/GuestログはRunner単位20MB×5世代、UI表示は直近5,000行です。これらは製品上の設定値であり、GitHub公式の制限ではありません。実機のディスク負荷を測定して変更できます。

export対象はアプリ/OS/WSLの版、状態snapshot、Operation履歴、redaction済みログ、設定の非秘密部分、採取時刻です。token、refresh token、Runnerの認証ファイル、private key、job workspace、ブラウザ情報を含めません。ユーザー名・repo名・ローカルパスも外部提供前にプレビューできるようにします。

診断exportを自動送信しません。外部telemetryはMVPでは無効・未導入を既定とします。ログをコピーすれば安全という保証はせず、機密が含まれ得るjob出力に注意を表示します。

## 4. アプリ更新とRunner更新を分離する

| 対象 | 管理方式 | 更新の注意 |
|---|---|---|
| GUI/Agent/Guest | 製品の署名済みリリース | protocol互換、busy、migration、rollbackを事前確認 |
| GitHub公式Runner | 公式の更新機構を尊重 | wrapperとプロセス再起動を追跡。更新通知をクラッシュ扱いしない |
| WSL/Ubuntu | 利用者承認によるOS管理 | 再起動や他作業への影響を説明。勝手にdistroを置換しない |
| 開発ツール/SDK | profile・workflowによる明示設定 | バージョン差を診断し、常時最新へ勝手に変更しない |

GitHub公式Runnerの自動更新を無効化する場合にはGitHub側の更新期限等を運用へ組み込む必要があるため、MVPでは安易に無効化しません。[S01](19_SOURCES.md#s01)

更新は「通知→互換確認→実行中処理の確認→安全な停止→署名検証→配置→migration→起動→health確認」を基本とします。任意のダウンロード先や署名を省略したforce updateを一般UIに公開しません。

## 5. 署名・配布

Tauri Updaterの署名とWindowsの実行ファイル/インストーラー署名は別物として扱います。片方を実装してもう片方も満たしたと表示しません。[S13](19_SOURCES.md#s13)

公開時には配布形式、署名鍵の保管、更新metadataのホスティング、失効時対応、担当者、再現可能なビルド手順を決めます。署名鍵をリポジトリや開発PCのjob環境へ置かず、リリース専用の保護された環境で扱います。

テスト専用WebDriver機能やdebug IPCをrelease artifactへ入れない検査を設けます。Tauri公式テスト案内は現在の仕組みを確認して用い、テスト用pluginを本番機能として公開しません。[S14](19_SOURCES.md#s14)

## 6. DBバックアップと復旧

AgentがSQLiteのsingle writerです。バックアップはDB接続から一貫したsnapshotとして取得し、WAL利用中の`.db`だけをコピーして完了としません。秘密本体は含めず、Credential参照が復元先で有効とは仮定しません。

migration前にschema version、互換性、空き容量、バックアップを確認します。新schemaで書込後に古いbinaryだけへ戻すことをrollbackと扱いません。対応する復元経路を用意できない変更は、更新を停止して利用者へ説明します。

## 7. 削除・アンインストール

削除計画には対象scope/remote ID、Runner領域、cache、local credentials、WSLへの影響を表示します。remote登録解除とローカル消去の結果を別々に記録し、ネット断で片方が残れば残件として提示します。

既存Ubuntuの`--unregister`、WSL全体のshutdown、同名サービス全削除、ユーザーの開発ディレクトリ消去は通常uninstallに含めません。専用distroであっても、利用者が保存したファイルがないか確認し、distro破棄は別の明示操作にします。[S06](19_SOURCES.md#s06)

GitHub認証の解除は、ローカルの秘密消去とGitHub側の承認取消しを区別します。ローカル消去だけでGitHub上の権限まで消えたと説明しません。

## 8. リリース判定表

| 項目 | 必須条件 |
|---|---|
| 技術検証 | G1〜G5の結果と対応環境を記録 |
| セキュリティ | P0脆弱性なし。残る同一ユーザー実行リスクを説明 |
| 品質 | Windows/WSL実機とMock試験を区別した試験記録 |
| 配布 | 署名、更新、互換、uninstall、test driver除外を確認 |
| 表示 | 実行前選択を完全フェイルオーバーと呼ばない |
| 依存 | SBOM/依存一覧、ライセンス、脆弱性確認 |
| プロジェクト | 製品名の商標・既存名確認、公開ライセンス決定、問い合わせ先 |
| 文書 | 制約、対応バージョン、認証・復旧手順、既知問題 |

未解決の公開準備項目があっても個人PoCは進められます。ただし正式公開済み・一般利用可能と称して配布しません。
