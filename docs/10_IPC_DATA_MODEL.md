# 10. IPC・データモデル・エラー契約

**区分:** 実装契約案 / **protocol major:** 1 / **DB schema:** 1

## 1. 通信面

Renderer→Tauriは限定command/event、Desktop Rust Bridge→AgentはWindows named pipe、Agent→WSL Guestは継続するstdioフレームを使います。ローカルWebサーバーの公開を標準構成にしません。

pipeの識別名は例として`\\.\pipe\localforge.<sid-hash>.v1`を用います。名前は認証の代わりではありません。DACL、所有SID、remote client拒否は[セキュリティ](09_SECURITY.md)の要件です。[S26](19_SOURCES.md#s26)

## 2. フレーム形式

`uint32 little-endian length + UTF-8 JSON payload`を基本とします。最大1MiB、長さ0・超過・不正UTF-8・未知majorは拒否します。読み取り途中の切断を完全なメッセージとして扱いません。

Guestのstdoutにはこのプロトコルだけを出します。Guestの診断はstderr、Runnerログは専用のLogEventで流します。依存コマンドのstdoutをそのまま親のstdoutへ流してはいけません。

## 3. メッセージ例

```json
{
  "protocolMajor": 1,
  "kind": "request",
  "requestId": "c917d45a-bf92-4ac0-8c3b-c0d731277658",
  "method": "node.start",
  "payload": {
    "nodeId": "node-home",
    "expectedRevision": 3
  }
}
```

```json
{
  "protocolMajor": 1,
  "kind": "response",
  "requestId": "c917d45a-bf92-4ac0-8c3b-c0d731277658",
  "ok": true,
  "result": {
    "operationId": "op-7c28",
    "accepted": true
  }
}
```

`accepted=true`は受付済みであって完了ではありません。実行結果はOperation snapshot/eventで返します。

```json
{
  "protocolMajor": 1,
  "kind": "event",
  "sequence": "1842",
  "agentGeneration": "9a9a6fec",
  "event": "runner.observation.changed",
  "payload": {
    "runnerId": "runner-linux",
    "local": "running",
    "remote": "unknown",
    "remoteErrorCode": "AUTH_EXPIRED",
    "verifiedAt": null,
    "desired": "running"
  }
}
```

## 4. メソッド一覧

| メソッド | 内容 | 副作用・注意 |
|---|---|---|
| `system.handshake` | protocol/実装版/能力 | 不一致時に操作を拒否する |
| `system.doctor` | 環境診断 | 原則読取のみ |
| `node.snapshot` | NodeとRunner観測 | 秘密を含めない |
| `node.start` | 起動要求 | Operation発行、冪等 |
| `node.stop` | idle待ち付き停止要求 | Unknown/Busyは保留 |
| `node.forceStop` | 強制停止 | confirmation challenge必須 |
| `runner.planCreate` | 変更計画 | GitHub/OSへの変更はまだしない |
| `runner.applyCreate` | 承認した計画を適用 | plan hash、期限、revisionを照合 |
| `runner.planRemove` | 削除対象一覧 | 所有リソースだけを対象にする |
| `runner.applyRemove` | 承認した対象を削除 | 登録解除失敗時にtombstone |
| `operation.get/cancel` | 操作監視・取消 | 外部副作用が取消済みとは限らない |
| `auth.beginDevice/cancel/logout` | 認証操作 | 通常DTOにtokenを返さない |
| `credential.import` | PAT入力の限定受渡し | 秘密payloadのログ・再送保存を禁止 |
| `events.subscribe` | 差分購読 | gapならsnapshotへ戻る |
| `logs.subscribe/export` | ログ閲覧・診断 | 機密除外、上限、出力先確認 |
| `settings.plan/apply` | 設定変更 | 影響とrestart要否を表示 |

汎用`exec(command)`、`readFile(path)`、`writeFile(path,data)`、`sql(query)`をpublic IPCとして提供しません。必要なfile選択は製品の許可済み操作へ変換します。

## 5. 型の決まり

時刻はUTC RFC3339、所要時間は単調時計を使ったミリ秒値、GitHub IDとevent sequenceは10進文字列とします。内部`u64`をJavaScript numberへ無検証に変換しません。

不明値は`null`または明示enumにし、`0`や`false`へ置換しません。表示用メッセージと機械判定用codeを分離し、日本語翻訳を変えても動作が変わらないようにします。

## 6. SQLiteの初期スキーマ例

以下は初期migrationの参考DDLです。GitHub資格情報やRunner固有credentialの中身は保存しません。AgentだけがDBを書きます。[S32](19_SOURCES.md#s32)

```sql
PRAGMA foreign_keys = ON;

CREATE TABLE nodes (
    id TEXT PRIMARY KEY,
    display_name TEXT NOT NULL,
    windows_sid TEXT NOT NULL,
    revision INTEGER NOT NULL DEFAULT 1
);
CREATE TABLE backends (
    id TEXT PRIMARY KEY,
    node_id TEXT NOT NULL REFERENCES nodes(id),
    kind TEXT NOT NULL CHECK (kind IN ('native_windows','wsl')),
    config_json TEXT NOT NULL,
    owned_by_product INTEGER NOT NULL CHECK (owned_by_product IN (0,1))
);
CREATE TABLE scopes (
    id TEXT PRIMARY KEY,
    github_host TEXT NOT NULL,
    kind TEXT NOT NULL CHECK (kind IN ('repo','org')),
    remote_id TEXT NOT NULL,
    display_path TEXT NOT NULL,
    UNIQUE (github_host, kind, remote_id)
);
CREATE TABLE credential_refs (
    id TEXT PRIMARY KEY,
    purpose TEXT NOT NULL,
    store_key TEXT NOT NULL UNIQUE,
    expires_at TEXT,
    permission_summary_json TEXT NOT NULL
);
CREATE TABLE runners (
    id TEXT PRIMARY KEY,
    backend_id TEXT NOT NULL REFERENCES backends(id),
    scope_id TEXT NOT NULL REFERENCES scopes(id),
    remote_runner_id TEXT,
    display_name TEXT NOT NULL,
    labels_json TEXT NOT NULL,
    install_path TEXT NOT NULL,
    desired_state TEXT NOT NULL CHECK (desired_state IN ('running','stopped','removed')),
    revision INTEGER NOT NULL DEFAULT 1,
    UNIQUE (scope_id, remote_runner_id)
);
CREATE TABLE operations (
    id TEXT PRIMARY KEY,
    request_id TEXT NOT NULL UNIQUE,
    target_id TEXT NOT NULL,
    operation_kind TEXT NOT NULL,
    phase TEXT NOT NULL,
    result_json TEXT,
    started_at TEXT NOT NULL,
    finished_at TEXT
);
CREATE TABLE managed_resources (
    id TEXT PRIMARY KEY,
    runner_id TEXT REFERENCES runners(id),
    kind TEXT NOT NULL,
    locator TEXT NOT NULL,
    ownership_proof_json TEXT NOT NULL,
    operation_id TEXT REFERENCES operations(id),
    state TEXT NOT NULL
);
CREATE TABLE audit_events (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    recorded_at TEXT NOT NULL,
    actor_sid TEXT NOT NULL,
    operation_id TEXT,
    event_kind TEXT NOT NULL,
    details_json TEXT NOT NULL
);
```

`config_json`等には別途version付きSchemaを適用します。JSON列があることは任意設定や任意コマンドを許可することを意味しません。

NodeからBackendを削除してもRunnerをcascadeで消さず、明示したoperationを通します。観測値はキャッシュであり、再起動時にlive確認せず信用しません。

## 7. トランザクションと回復

外部API呼出しとSQLite transactionを1つの原子操作として扱いません。まずOperationと意図を保存し、外部処理を実施し、結果を記録します。クラッシュ後は途中phaseと所有リソースから再照合します。

DB migration前には整合性のあるバックアップを作ります。WAL使用中の本体ファイルだけをコピーしない方式にし、SQLite backup機構または安全な停止状態で保存します。migration失敗時は旧版を壊さず起動を止め、回復手順へ誘導します。

## 8. エラー契約

```json
{
  "code": "WSL_GUEST_UNREACHABLE",
  "messageKey": "errors.wslGuestUnreachable",
  "retryable": true,
  "requiresConfirmation": false,
  "operationId": "op-7c28",
  "details": {"backendId": "backend-ubuntu"}
}
```

主なcodeは`AUTH_EXPIRED`、`PERMISSION_OR_POLICY`、`RATE_LIMITED`、`RUNNER_BUSY`、`STATUS_STALE`、`PATH_NOT_OWNED`、`PROTOCOL_MISMATCH`、`REVISION_CONFLICT`、`REQUIRES_CONFIRMATION`、`CHECKSUM_MISMATCH`です。生HTTP応答やOSコマンドラインをそのままdetailsに入れません。

## 9. 配信・互換性

購読者ごとに上限付きキューを持ち、遅いGUIがAgent全体を止めないようにします。重要状態イベントを落とした場合は`ResyncRequired`を送り、ログを間引いた場合は件数を表示します。

未知のメッセージ種別やmajor不一致を無視して誤操作するより、安全に接続拒否します。minor追加はunknown field許容で互換性を保ち、削除・型変更はmajorまたは移行期間を設けます。
