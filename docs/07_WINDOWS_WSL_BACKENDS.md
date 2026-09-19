# 07. Windows / WSL Backend実装仕様

**区分:** 実装設計 / **実機検証必須:** LF-003、LF-004、LF-007、LF-008

## 1. 既存環境を壊さない原則

Runnerのインストール先、Windowsプロセス、WSLディストリビューション、Linuxユーザー、GitHub登録には所有権情報を残します。`actions.runner.*`のようなワイルドカードで他のRunnerまで操作しません。

既存のWindowsサービス版・systemdサービス版Runnerを見つけても勝手に移管しません。MVPでは読取診断と警告までです。同じRunnerディレクトリにサービスと新Agentを同時に接続させません。

## 2. 構築の前提チェック

| Windows | WSL/Linux |
|---|---|
| Windows 11 x64、実行ユーザーSID | WSLの版、対象distroのWSL1/2区別 |
| 実行先・作業先の書込権限 | `/etc/os-release`とCPU architecture |
| 必要な空き容量、ネットワーク | 作業先がLinux側filesystemにあること |
| 他Agent/Runnerとの衝突 | 非root実行ユーザーとhomeの存在 |
| WebView2等の本体前提 | 必要runtime依存の不足と導入計画 |

WSL CLIの一覧出力はローカライズ、UTF-16/NUL、空白を含む名前などを想定してfixtureを作ります。表示用一覧の文字列だけでWSL2・OS版・ユーザーを判定しません。[S06](19_SOURCES.md#s06)

## 3. 保存場所

提案配置は次のとおりです。公開前に名称とインストーラー方式を確定します。

```text
Windows:
  %LOCALAPPDATA%\LocalForge\state\localforge.db
  %LOCALAPPDATA%\LocalForge\runners\<runner-id>\
  %LOCALAPPDATA%\LocalForge\cache\<version>-<arch>\
  %LOCALAPPDATA%\LocalForge\logs\

WSL:
  /home/<ci-user>/.local/share/localforge/runners/<runner-id>/
  /home/<ci-user>/.local/share/localforge/bin/localforge-guest
```

Ubuntu Runnerの`_work`を既定で`/mnt/c`へ置きません。WindowsとLinuxのファイル権限・改行・path・I/O特性を混同しないための製品方針です。ユーザーによる保存先指定には診断と警告を付けます。

## 4. Runner取得・登録

GitHub公式の配布物だけを対象にし、TLS、取得元、architecture、公開されたchecksumを検証します。checksumが取得できない・不一致の場合、黙って検証を省略しません。GitHubと製品配布サーバーの両方を信頼する設計へ拡大するときは脅威モデルを見直します。

アーカイブの展開は一時ディレクトリで行い、パストラバーサル、symlinkによる逸脱、展開サイズ上限を検証後に配置します。設定は公式`config.cmd`/`config.sh`、起動は公式`run.cmd`/`run.sh`を使う方針です。[S21](19_SOURCES.md#s21)

自己更新が書き換えるファイルを複数Runnerで共有しません。配布物cacheは読取用とし、各Runnerへ別の実行ディレクトリを用意します。

## 5. Windows起動

Agentは通常権限で公式Runnerラッパーを起動し、親子プロセスを追跡します。Job Objectの利用、signalの送り方、強制終了時の子孫処理をPoCで検証します。[S25](19_SOURCES.md#s25)

`.cmd`の呼出しではシェルの解釈が介在するため、製品管理下の固定ラッパーと検証済み引数生成を使います。repo名・label・ユーザー入力を未検証で`cmd /c`へ連結しません。空白、日本語、引用符、`&`を含むpathの扱いをテストします。

GUIを閉じる操作はAgent/Job Objectのhandleを閉じません。Agent自身の終了時だけ、明示したfail-stopまたは再接続方針を適用します。

## 6. WSLの起動維持

WSL Backendは、Windows側から継続する`wsl.exe`セッションでGuestを起動します。Guestが公式Runnerラッパーを子として管理し、管理通信のstdoutとRunnerの出力を混ぜません。

```text
Windows Agent
    │ 長寿命のstdin/stdoutフレーム
wsl.exe --distribution <managed-distro> --user <ci-user> --exec <guest>
    │
Linux Guest
    └─ run.sh → GitHub公式Runner
```

**`wsl.exe ... /bin/true`だけで起こして終える方式は採用しません。** systemdサービスのみではWSLを存続させないというMicrosoftの説明を踏まえ、Guestセッションでの維持を実機検証します。[S04](19_SOURCES.md#s04)

Guestに管理用GitHub PATを渡しません。起動、状態、停止などの限定操作だけを実装し、bootstrap時のroot操作と通常実行ユーザーを分けます。

## 7. systemdの位置づけ

MVPの新規RunnerはAgent-managedとし、systemdサービス化を必須にしません。これにより常駐監督者をAgent/Guestへ一本化します。既存のservice-managed Runnerを取り込む機能は後続です。

systemdが利用可能な環境では、将来のresource scopeやservice管理の候補になります。存在しないunitや名前の部分一致にfallbackせず、管理対象unitのexact nameを記録する設計にします。

## 8. ディストリビューションの導入

既存WSL2 Ubuntuの使用を許可しますが、CI専用distroを推奨します。未導入なら公式手順を案内し、WSL feature有効化、Linux初期ユーザー作成、再起動を含む計画として提示します。[S06](19_SOURCES.md#s06)

自動importを実装する場合、rootfsの公式取得先と整合性確認、ライセンス・再配布条件、ディスク容量、途中失敗の回復を先に検証します。未知のrootfsをインターネットから探して実行しません。

「初回から一切の確認・再起動なしで1クリック構築」を保証しません。**セットアップ後の通常起動が1操作**であることを製品の基準にします。

## 9. 停止と削除

通常のStop Nodeは対象Runnerへの停止要求とGuestの終了までです。WSLディストリビューションそのものの終了は既定OFFとし、専用・製品管理対象と確認できた場合だけ別の確認付き操作にします。

`wsl --shutdown`は全WSLを巻き込むため通常停止に使用しません。`--terminate <name>`でもそのdistro内の別作業を止めるため、Runner停止と同じ意味にはしません。[S06](19_SOURCES.md#s06)

`wsl --unregister`はデータ削除操作です。製品管理対象の所有権確認、稼働確認、バックアップ案内、明示確認がない限り実行しません。アンインストールの既定ではdistroを削除しません。

## 10. 資源制約

`.wslconfig`のCPU/RAM設定はWSL2全体の設定です。専用Ubuntuだけの上限としてUI表示しません。MVPは読取・影響説明・設定案の書出しまでとし、既存設定の自動上書きをしません。[S05](19_SOURCES.md#s05)

Runnerごとの制限はWindows Job ObjectとLinux cgroup等を別途検証します。厳密な制限ができない段階では「推奨値」「soft limit」「OS全体設定」を区別します。

## 11. スリープ・再起動・障害

スリープ中にCIを継続できるとは扱いません。実行中のスリープ抑止は利用者が許可した場合だけ導入し、元の電源設定を勝手に変えません。復帰後はLocal/Remote両方をUnknownから再照合します。

Guest切断、`wsl --terminate`の外部実行、Agent kill、OS更新再起動、Runner自己更新、ディスク満杯を試験項目にします。自動再起動は回数上限とbackoff付きで、失敗ループ中の「online」表示を禁止します。
