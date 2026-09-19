# 14. 実装バックログ

**区分:** 提案 / **状態:** 全件未着手 / **粒度:** 1件から複数の小さなPRへ分割可

これはGitHub/Linearへ起票可能な作業定義です。実際のIssue・ブランチ・PRは作成していません。番号は識別子であり、実装順序ではありません。各作業の「対象」は基盤・技術検証として関与する要件を含む**関連要件**です。要件の実装責任を持つタスクは[追跡表](20_TRACEABILITY.md)を正とします。各作業は[タスク雛形](../templates/IMPLEMENTATION_TASK.md)を使い、受入条件と実施した試験を記録します。

## 1. 最初に着手する作業

### LF-001: 再現可能なworkspaceを作る

- **段階:** P0 / **依存:** なし / **対象:** NFR-003
- **成果物:** `apps/desktop`、core/protocol/agent/guest/CLIの最小crate、frontend workspace、lockfile、`versions.md`、Mock起動経路。
- **作業:** [技術選定](04_TECHNOLOGY_STACK.md)の候補を実際に互換確認し、exact versionを固定。Windowsで空のTauri UIが起動し、Linuxで純粋coreの単体テストが動くところまで。
- **完了条件:** 新規cloneから文書のコマンドで再現できる。GitHub認証・Runner登録・WSL変更なしで起動する。CIはhostedを基本にし、開発中の本ソフトを必須の実行環境にしない。
- **検証:** TC-041/045へ向けたビルド基盤。製品E2Eが通ったとは記載しない。

### LF-002: ドメイン、IPC契約、MockBackendを作る

- **段階:** P0 / **依存:** LF-001 / **対象:** FR-008/009/013、NFR-006/008/011
- **成果物:** Node/Backend/Runner/Operation、希望状態と観測状態、エラー、schema、生成TS、固定seedのMock。
- **作業:** [状態定義](05_DOMAIN_STATE.md)と[IPC仕様](10_IPC_DATA_MODEL.md)を実装。開始・停止・busy・stale・API障害・部分失敗をUI未接続でも再現する。
- **完了条件:** `busy:false`とmissingを区別。重複要求は同じOperationに収束。Agent世代が変わったとき旧イベントを排除できる。契約生成物の差分検出をCIへ入れる。
- **検証:** TC-003/006/021/026/036。

### LF-003: Windows公式Runnerのプロセス管理を技術検証する

- **段階:** P0 / **依存:** LF-001/002 / **対象:** FR-004/006/007/014
- **成果物:** 非昇格の起動・停止・監視PoC、OS情報付き検証記録、継続/棄却ADR。
- **作業:** テスト用の公式Runnerで`run.cmd`経由の起動、標準出力、ジョブ終了、Agent死亡時、公式更新時の子プロセスを調べる。Job Objectの適用範囲を実測する。[S25](19_SOURCES.md#s25)
- **完了条件:** 自分が起動したプロセス木だけを制御できる。通常終了と強制終了を分離。更新の親子関係・終了コードを確認し、未知の挙動は診断可能な状態として止める。
- **検証:** TC-004/012/014/016/022/040。実Runnerを使う場合は利用者の承認と専用private repoが必要。

### LF-004: WSLの起動維持とGuest寿命を技術検証する

- **段階:** P0 / **依存:** LF-001/002 / **対象:** FR-005/006/009/015
- **成果物:** `wsl.exe`長寿命接続、非root Guest、protocol/stdout分離、切断時の挙動記録。
- **作業:** GUI終了後もAgentとGuestが維持され、WSLのアイドル停止に依存しないことを確認する。WSL停止、ネット断、Windowsスリープ復帰も注入する。`wsl.exe --exec`の長寿命stdioについて、stdin/stdoutのバッファリング、バイナリ透過性(改行・NUL・文字コード変換の有無)、1MiB近いframe、無通信が続いた後の往復も確認する。
- **完了条件:** 管理対象Runnerだけを起動/停止。無関係のdistroへ影響しない。1時間アイドル試験を記録。systemdサービスだけによる常駐を合格としない。[S04](19_SOURCES.md#s04)
- **検証:** TC-005/014/015/016/019/035。

### LF-005: IPCとTauri権限境界を実装する

- **段階:** P0 / **依存:** LF-002 / **対象:** NFR-001/005/006/011
- **成果物:** SIDに基づくnamed pipe、ACL、ローカル接続限定、固定method registry、frame上限、単一起動。[S11](19_SOURCES.md#s11)[S26](19_SOURCES.md#s26)
- **完了条件:** 別SID・無許可command・過大frameを拒否。raw shell、任意ファイル書込み、直接SQL等を公開しない。再接続でsnapshotを回復する。GUIから起動したAgentが、GUIのプロセスツリー・Job Object・コンソールから切り離され、GUIの終了・強制終了・更新後も存続する。
- **検証:** TC-001/025/026/036。

### LF-006: GitHub App Device Flowと更新を検証する

- **段階:** P0 / **依存:** LF-001 / **対象:** FR-002、NFR-004/012
- **成果物:** テストGitHub Appでの認証・期限・refresh・revokeの記録、権限表、認証ADR。
- **作業:** Device Flow有効化、App installation、userとの権限の交差を確認。新規認証だけでなくdevice-flow由来tokenのsecret不要refreshを検証する。[S08](19_SOURCES.md#s08)[S09](19_SOURCES.md#s09)
- **完了条件:** 配布バイナリにclient secret/private keyを入れない。認可待ち・slow_down・取消し・期限切れを正しく処理。MVPのlocal用tokenとWorkflow用read-only PATを混同しない。
- **検証:** TC-020/024/039/040。App登録や秘密の発行は承認を得て行う。

## 2. MVP実装タスク

| ID | 作業 | 依存 | 完了条件 | 主な受入テスト |
|---|---|---|---|---|
| LF-007 | Windows Runner準備・登録 | 003/005/006/012 | 取得元・hash・展開先を検証。config後にremote IDを保存し、中断から重複せず復旧する | TC-016/017/021/037 |
| LF-008 | WSL Runner準備・登録 | 004/005/006/012 | distro/利用者/保存先を確認。専用領域へ配置し非rootで稼働。再実行で既存データを消さない | TC-015/016/017/019/037 |
| LF-009 | Node一括操作・reconciler | 007/008 | 片方失敗を隠さず、再試行・取消し・busy保留・強制停止確認を実装する | TC-001/012/013/014/021/034/035 |
| LF-010 | Dashboard・setup wizard | 002/005 | Mockで全状態を表示し、Operation受理と完了を区別。GUIを閉じてもAgentを停止しない | TC-002/006/038 |
| LF-011 | GitHub remote health | 006/012 | exact ID、freshness、Unknown、ETag、pagination、backoff、権限エラーを実装する | TC-003/007/008/036/039 |
| LF-012 | Credential adapterとローテーション | 002/006 | Windows秘密保存、期限、単一refresh、削除、最小env継承。平文をDB・ログに入れない | TC-020/023/024/040 |
| LF-013 | ログ・診断export | 002/005 | 上限、ローテーション、redaction、correlation、保存先案内。長大ログでUIを止めない | TC-023/026/044 |
| LF-014 | Workflow事前選択generator | 002/011 | 純粋関数とfixture。trusted/private/ref検査を先に行い、bool falseのみ受入れ、課金許可を尊重する | TC-003/007/009/010/011/028 |
| LF-015 | Workflow Assistant画面 | 010/014 | 新規YAMLの説明・必要Secret・差分/保存先を確認。MVPでは既存workflowの自動上書きをしない | TC-009/028 |
| LF-016 | 最小CLI | 002/005/009/011 | doctor/status/start/stopをGUIと同じ契約から実行。JSON出力に秘密を含めない | TC-023/042 |
| LF-017 | 設定とログオン時起動 | 005/009/010 | 同意付きuser-session起動。非昇格・複数起動防止。ログアウト後継続を約束しない | TC-001/004/043 |
| LF-018 | 削除・中断・復旧 | 007/008/009 | exact所有領域だけ削除。offlineのremote登録残存を記録し、再試行可能にする | TC-018/021/027/031/037 |
| LF-019 | 配布・更新・schema互換 | 005/009/018 | 署名検証、UI/Agent/Guest互換判定、更新中busy保護、migrationの復旧経路を持つ | TC-022/027/030/045 |
| LF-020 | セキュリティ横断試験 | 005/012/013/014/018 | threat modelのP0対策を実証。自宅PCのsandbox保証をUIでしない | TC-009/016/017/018/023/024/025/026/040 |
| LF-021 | 実機統合試験 | 009/010/011/015/016/017/029 | 同じprivate repoでWindows/Ubuntu/hosted分岐、GUI再接続、スリープ復帰を記録 | TC-002/005/010/011/035/041/042/043/044 |
| LF-022 | MVP公開判定 | 019/020/021 | 残件・非対応・runbook・ライセンス・名称・署名・テスト証跡を確認。未解決P0を隠さない | TC-030/031/038/045 |
| LF-029 | 診断・軽量metrics | 002/003/004/005 | WSL/OS前提、CPU/RAM/disk、測定不可を区別。全PC負荷をRunner固有値と誤表示しない | TC-037/044 |

依存欄の`003`は`LF-003`の略記です。LF-012はLF-006の検証用最小adapterを拡張するため、006が012の完成に依存する循環を作りません。

## 3. MVP後のタスク

| ID | 段階 | 作業 | 開始条件・受入条件 |
|---|---|---|---|
| LF-023 | P1 | Organization・複数repo | 既存のscope ID設計を利用。org権限、同名Runner、paginationをTC-008/033で確認 |
| LF-024 | P1 | 既存workflowのCST差分編集 | コメント・式・needsを保持。読取hash競合をTC-029で拒否。複雑構文は自動変換しない |
| LF-025 | P1 | リソース上限・プロファイル | `.wslconfig`全体影響とWindows側制限の差を説明。TC-019/034を拡張する |
| LF-026 | P2 | Docker・使い捨て環境 | ephemeral Runner登録と実行環境破棄を別機能にする。TC-032で検証 |
| LF-027 | P2/任意 | 認証・selector用broker | ローカル停止時も利用可能なサーバーの運用費・脅威・可用性を別ADRで承認。MVP前提にしない |
| LF-028 | P1 | 既存Runnerの取込み | サービス・既存親プロセス・所有者を検出し、二重supervisorを禁止。TC-015/022/031を拡張 |

## 4. Issueの共通Definition of Done

受入条件を満たし、追加テストを実行し、未実施試験を明記し、FR/TCを更新してから完了にします。仕様と異なる実装は暗黙に正とせずADRを付けます。外部サービスに実変更した場合は、その対象・同意・後片付けを報告します。

成果物がPoCなら「本番品質」と記載しません。技術検証でNo-Goになった結果にも価値があるため、失敗の再現条件と代替案を残します。
