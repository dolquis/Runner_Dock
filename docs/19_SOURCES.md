# 19. 根拠資料と確認範囲

**基準日:** 2026-09-19 / **資料方針:** GitHub、Microsoft、採用技術の公式文書と参考実装の原文を優先

本文の`Sxx`は、このページの安定アンカーへリンクします。外部文書の内容は変わり得るため、実装開始時・リリース前に再確認します。参照先の更新日と本書の確認日を混同しません。

## 事実・設計・未検証の区別

外部APIの権限やWSLの制約は下記資料に基づきます。一方、Nodeモデル、Guest方式、IPC schema、要件、UI、テスト、バックログはこのプロジェクト向けの設計案です。公式資料の掲載だけで、その設計全体が動作保証されるわけではありません。

HomeRunは公開説明と指定ファイルを確認した参考対象です。コード全体の監査や実行はしていません。競合の完全調査、名称・商標確認、価格保証も含みません。

## 出典一覧

<a id="s01"></a>
### S01: GitHub: Self-hosted runners reference

**原文:** [GitHub: Self-hosted runners reference](https://docs.github.com/en/actions/reference/runners/self-hosted-runners)  
**確認日:** 2026-09-19  
**使用範囲:** 対応環境、ラベル割当、キュー、公式Runner更新、ephemeral、Scale Set Client。ローカル状態照会による予約やhostedへの自動切替を保証する資料ではない。

<a id="s02"></a>
### S02: GitHub REST: Self-hosted runners

**原文:** [GitHub REST: Self-hosted runners](https://docs.github.com/en/rest/actions/self-hosted-runners)  
**確認日:** 2026-09-19  
**使用範囲:** repo/org Runnerの一覧・個別取得、online/busy、registration/remove token、権限read/write。アプリの選択ロジックは独自設計。

<a id="s03"></a>
### S03: GitHub REST: API versions

**原文:** [GitHub REST: API versions](https://docs.github.com/en/rest/about-the-rest-api/api-versions)  
**確認日:** 2026-09-19  
**使用範囲:** 2026-03-10のAPIバージョンを確認。HTTPクライアントで明示し、将来の更新は互換検証する。

<a id="s04"></a>
### S04: Microsoft: WSL and systemd

**原文:** [Microsoft: WSL and systemd](https://learn.microsoft.com/en-us/windows/wsl/systemd)  
**確認日:** 2026-09-19  
**使用範囲:** systemdの有効化と、systemdサービスだけではWSLインスタンスを維持しないという注意点。Guest方式そのものの動作保証ではない。

<a id="s05"></a>
### S05: Microsoft: Advanced settings configuration in WSL

**原文:** [Microsoft: Advanced settings configuration in WSL](https://learn.microsoft.com/en-us/windows/wsl/wsl-config)  
**確認日:** 2026-09-19  
**使用範囲:** Windows側.wslconfigはWSL2全体、/etc/wsl.confはdistro側の設定。メモリ/CPU等の設定と反映のための再起動。

<a id="s06"></a>
### S06: Microsoft: Basic commands for WSL

**原文:** [Microsoft: Basic commands for WSL](https://learn.microsoft.com/en-us/windows/wsl/basic-commands)  
**確認日:** 2026-09-19  
**使用範囲:** 列挙、実行、distro指定、terminate、shutdown、import/export、unregister。破壊操作と起動操作を区別する根拠。

<a id="s07"></a>
### S07: Microsoft: WSL FAQ

**原文:** [Microsoft: WSL FAQ](https://learn.microsoft.com/en-us/windows/wsl/faq)  
**確認日:** 2026-09-19  
**使用範囲:** Windows/Linuxのファイル・相互運用とWSLの前提。WSLを任意コードの完全隔離環境として扱わない。

<a id="s08"></a>
### S08: GitHub: Generating a user access token for a GitHub App

**原文:** [GitHub: Generating a user access token for a GitHub App](https://docs.github.com/en/apps/creating-github-apps/authenticating-with-a-github-app/generating-a-user-access-token-for-a-github-app)  
**確認日:** 2026-09-19  
**使用範囲:** Appのuser権限、Device Flow、code交換とclient_secret/PKCE、期限。実Appの権限構成は未検証。

<a id="s09"></a>
### S09: GitHub: Refreshing user access tokens

**原文:** [GitHub: Refreshing user access tokens](https://docs.github.com/en/apps/creating-github-apps/authenticating-with-a-github-app/refreshing-user-access-tokens)  
**確認日:** 2026-09-19  
**使用範囲:** device-flow由来tokenのrefreshではclient_secret不要という条件、refreshによるtoken置換、期限。LF-006で実証する。

<a id="s10"></a>
### S10: GitHub: Authorizing OAuth apps

**原文:** [GitHub: Authorizing OAuth apps](https://docs.github.com/en/apps/oauth-apps/building-oauth-apps/authorizing-oauth-apps)  
**確認日:** 2026-09-19  
**使用範囲:** OAuth Appのフローとcallbackの資料。GitHub Appとは登録種類・token要件を区別するため参照。

<a id="s11"></a>
### S11: Tauri 2: Capabilities

**原文:** [Tauri 2: Capabilities](https://v2.tauri.app/security/capabilities/)  
**確認日:** 2026-09-19  
**使用範囲:** WebView/windowに付与する権限の境界。capability導入だけで自作commandが自動的に安全になるとは解釈しない。

<a id="s12"></a>
### S12: Tauri 2: Prerequisites

**原文:** [Tauri 2: Prerequisites](https://v2.tauri.app/start/prerequisites/)  
**確認日:** 2026-09-19  
**使用範囲:** Windows開発環境のC++ツールとWebView2等の前提。詳細workloadは導入日に再確認する。

<a id="s13"></a>
### S13: Tauri 2: Updater

**原文:** [Tauri 2: Updater](https://v2.tauri.app/plugin/updater/)  
**確認日:** 2026-09-19  
**使用範囲:** Updaterの署名・設定・配布設計。Windowsコード署名とは別に扱う。

<a id="s14"></a>
### S14: Tauri 2: WebDriver testing

**原文:** [Tauri 2: WebDriver testing](https://v2.tauri.app/develop/tests/webdriver/)  
**確認日:** 2026-09-19  
**使用範囲:** 現行のWebdriverIO/Tauri test serviceを含むE2E案内。実アプリ試験とブラウザMock試験を区別し、テスト専用機能をreleaseへ含めない。

<a id="s15"></a>
### S15: Rust official blog: release announcements

**原文:** [Rust official blog: release announcements](https://blog.rust-lang.org/)  
**確認日:** 2026-09-19  
**使用範囲:** 確認時点のRust 1.98.1 stable告知を初期候補の根拠とした。採用toolchainはLF-001で固定。

<a id="s16"></a>
### S16: React: Versions

**原文:** [React: Versions](https://react.dev/versions)  
**確認日:** 2026-09-19  
**使用範囲:** 確認時点の最新系列19.3を記録。プロジェクトでの組合せ互換は別途検証。

<a id="s17"></a>
### S17: Vite: Releases and support

**原文:** [Vite: Releases and support](https://vite.dev/releases)  
**確認日:** 2026-09-19  
**使用範囲:** 確認時点の8.3系が通常修正対象。過去会話の8.1固定案を更新。

<a id="s18"></a>
### S18: Node.js: Previous releases

**原文:** [Node.js: Previous releases](https://nodejs.org/en/about/previous-releases)  
**確認日:** 2026-09-19  
**使用範囲:** 確認時点の24 LTSを開発用runtime候補とした。Currentを無条件に採用しない。

<a id="s19"></a>
### S19: GitHub Actions: Secure use reference

**原文:** [GitHub Actions: Secure use reference](https://docs.github.com/en/actions/reference/security/secure-use)  
**確認日:** 2026-09-19  
**使用範囲:** persistent self-hosted環境と信頼できないコード・権限のリスク。private repoでも秘密隔離の保証にはならない。

<a id="s20"></a>
### S20: GitHub billing: GitHub Actions

**原文:** [GitHub billing: GitHub Actions](https://docs.github.com/en/billing/concepts/product-billing/github-actions)  
**確認日:** 2026-09-19  
**使用範囲:** self-hosted利用、標準public hosted、privateの利用枠や超過・保存に関する課金説明。価格を固定せず、selectorにも費用が生じ得ることを示す。

<a id="s21"></a>
### S21: GitHub: Adding self-hosted runners

**原文:** [GitHub: Adding self-hosted runners](https://docs.github.com/en/actions/how-tos/manage-runners/self-hosted-runners/add-runners)  
**確認日:** 2026-09-19  
**使用範囲:** 公式Runnerの導入・登録、scope、tokenの有効期間、Windowsサービス登録時の権限など。

<a id="s22"></a>
### S22: GitHub Actions: Workflow syntax

**原文:** [GitHub Actions: Workflow syntax](https://docs.github.com/en/actions/reference/workflows-and-actions/workflow-syntax)  
**確認日:** 2026-09-19  
**使用範囲:** runs-on、needs/outputs、式、workflow_dispatch、job条件、timeout等の仕様。生成例の実行確認ではない。

<a id="s23"></a>
### S23: GitHub REST: Best practices

**原文:** [GitHub REST: Best practices](https://docs.github.com/en/rest/using-the-rest-api/best-practices-for-using-the-rest-api)  
**確認日:** 2026-09-19  
**使用範囲:** ETag等の条件付き取得、rate limit、効率的なAPI呼出し、権限がない場合の404の扱い。

<a id="s24"></a>
### S24: GitHub REST: Pagination

**原文:** [GitHub REST: Pagination](https://docs.github.com/en/rest/using-the-rest-api/using-pagination-in-the-rest-api)  
**確認日:** 2026-09-19  
**使用範囲:** 一覧APIのページ追跡とLinkヘッダー。ページ未完了を全件取得として扱わない。

<a id="s25"></a>
### S25: Microsoft: Job Objects

**原文:** [Microsoft: Job Objects](https://learn.microsoft.com/en-us/windows/win32/procthread/job-objects)  
**確認日:** 2026-09-19  
**使用範囲:** Windowsプロセス群の管理・制限の基盤。公式Runner更新と組み合わせた挙動は実機検証が必要。

<a id="s26"></a>
### S26: Microsoft: Named Pipe Security and Access Rights

**原文:** [Microsoft: Named Pipe Security and Access Rights](https://learn.microsoft.com/en-us/windows/win32/ipc/named-pipe-security-and-access-rights)  
**確認日:** 2026-09-19  
**使用範囲:** Windows named pipeのアクセス制御。pipe名だけを認証や秘密にしない設計の根拠。

<a id="s27"></a>
### S27: Microsoft: Credentials Management

**原文:** [Microsoft: Credentials Management](https://learn.microsoft.com/en-us/windows/win32/secauthn/credentials-management)  
**確認日:** 2026-09-19  
**使用範囲:** Windows資格情報の管理API。アプリとjobが同一ユーザーの場合の完全隔離を保証する資料ではない。

<a id="s28"></a>
### S28: HomeRun: Repository / README

**原文:** [HomeRun: Repository / README](https://github.com/aGallea/homerun)  
**確認日:** 2026-09-19  
**使用範囲:** 参考実装の公開説明。現在機能の網羅監査・他製品不在の証明ではない。

<a id="s29"></a>
### S29: HomeRun: ARCHITECTURE.md

**原文:** [HomeRun: ARCHITECTURE.md](https://github.com/aGallea/homerun/blob/master/docs/ARCHITECTURE.md)  
**確認日:** 2026-09-19  
**使用範囲:** GitHub connectorで本文を確認。文書内にmacOS/Unix socket中心の説明が残る。確認したblob SHA: 485f4e6eef525dc9dcaf5bed735bb635bc504261。これはcommit SHAではない。

<a id="s30"></a>
### S30: HomeRun: LICENSE

**原文:** [HomeRun: LICENSE](https://github.com/aGallea/homerun/blob/master/LICENSE)  
**確認日:** 2026-09-19  
**使用範囲:** GitHub connectorでMIT Licenseを確認。blob SHA: ce13dfbc02574d2c36e5a38f2336bd4b28c9b07e。ユーザーの新規プロジェクトのライセンスを指定するものではない。

<a id="s31"></a>
### S31: Tokio official documentation

**原文:** [Tokio official documentation](https://tokio.rs/)  
**確認日:** 2026-09-19  
**使用範囲:** Rust非同期runtimeの基礎資料。crateのexact versionと動作保証は初期build時に固定する。

<a id="s32"></a>
### S32: SQLx crate documentation

**原文:** [SQLx crate documentation](https://docs.rs/sqlx/latest/sqlx/)  
**確認日:** 2026-09-19  
**使用範囲:** SQLx API・DB接続の一次資料。掲載DDLは本プロジェクト独自の設計例。

<a id="s33"></a>
### S33: TanStack Query: React overview

**原文:** [TanStack Query: React overview](https://tanstack.com/query/latest/docs/framework/react/overview)  
**確認日:** 2026-09-19  
**使用範囲:** Reactで非同期状態を取得・cacheするための候補。Agent eventとの統合は独自設計。

<a id="s34"></a>
### S34: GitHub Actions: Limits

**原文:** [GitHub Actions: Limits](https://docs.github.com/en/actions/reference/limits)  
**確認日:** 2026-09-19  
**使用範囲:** Actionsのjob、queue、運用上の制限。自作アプリのtimeoutや同時listener設定と区別する。

## 未確認の外部条件

実GitHub Appの登録・認証更新、現物Windows/WSLでのプロセス寿命、実Runner更新、GitHub Actions上での生成Workflow実行、署名証明書と公開ライセンス、製品名の利用可能性は未確認です。それぞれLFタスクとゲートに割り当てています。

このパッケージに含まれるサンプルは、外部公式コードの転載ではなく、本プロジェクト用の説明・試験用例です。掲載例を製品コードへ移す際は、同じ受入試験と安全性レビューを通してください。
