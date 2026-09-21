// このファイルは crates/protocol から生成する。手で編集しない。
// 再生成: cargo run -p runnerdock-protocol --bin contracts-gen
// 差分検査: cargo run -p runnerdock-protocol --bin contracts-gen -- --check

/** ワイヤー契約の major。不一致なら接続しない。 */
export const PROTOCOL_MAJOR = 1;

/**
 * Agent の世代。Agent が再起動すると変わり、旧世代の event を捨てるのに使う。
 */
export type AgentGeneration = string;

/**
 * Desktop shell が UI へ返す Agent の状態。
 *
 * Renderer と Tauri の境界に出る型もここへ置く。手で写した型を UI 側に作ると
 * 契約が 2 か所になるためで、生成の仕組みは ADR-017 のものをそのまま使う。
 * UI は pipe 名も実行ファイルの path も受け取らない。取得できなかったことは
 * `Result` の失敗側で表し、既定値で埋めない。
 */
export interface AgentStatus {
  readonly handshake: HandshakeResult;
  readonly snapshot: NodeSnapshot;
}

/**
 * Backend（`NativeWindows` または具体的な WSL 環境）の ID。
 */
export type BackendId = string;

/**
 * Backend の種別。
 *
 * ワイヤー表現は `docs/10_IPC_DATA_MODEL.md` §6 の
 * `kind IN ('native_windows','wsl')` と一致させる。
 */
export type BackendKind = "native_windows" | "wsl";

/**
 * 秘密本体を含まない資格情報参照の ID。
 */
export type CredentialRefId = string;

/**
 * 秘密本体を含まない資格情報の表示用ビュー。
 */
export interface CredentialRefView {
  readonly credentialRefId: CredentialRefId;
  readonly expiresAt?: Timestamp | null;
  readonly permissionSummary: readonly string[];
  /**
   * 用途。ローカル管理用と Workflow 監視用を混同しない。
   */
  readonly purpose: string;
}

/**
 * 符号なし 64bit 整数を 10 進文字列で表したもの
 */
export type DecimalU64 = string;

/**
 * 利用者の意図。進行中の操作とは別に保持する。
 */
export type DesiredState = "running" | "stopped" | "removed";

/**
 * UI へ出す有効状態。`docs/05_DOMAIN_STATE.md` §4 の表に対応する。
 */
export type EffectiveState = "busy" | "unknown" | "ready" | "disconnected" | "reconciling" | "monitoring_unavailable" | "partial";

/**
 * 機械判定用のエラー code。日本語訳を変えても動作が変わらないようにする。
 */
export type ErrorCode = "PERMISSION_OR_POLICY" | "RATE_LIMITED" | "RUNNER_BUSY" | "PROTOCOL_MISMATCH" | "REQUIRES_CONFIRMATION" | "CHECKSUM_MISMATCH" | "WSL_GUEST_UNREACHABLE" | "AUTH_EXPIRED" | "STATUS_STALE" | "PATH_NOT_OWNED" | "REVISION_CONFLICT" | "REQUEST_ID_CONFLICT" | "METHOD_NOT_SERVED";

/**
 * ワイヤー上のエラー payload。
 */
export interface ErrorPayload {
  readonly code: ErrorCode;
  /**
   * 構造化した補足。秘密・生応答・コマンドラインを入れない。
   */
  readonly details?: { readonly [key: string]: unknown };
  /**
   * UI 側の翻訳キー。表示文字列そのものではない。
   */
  readonly messageKey: string;
  readonly operationId?: OperationId | null;
  readonly requiresConfirmation: boolean;
  readonly retryable: boolean;
}

/**
 * Agent が push する差分イベント。
 */
export interface Event {
  /**
   * 発行元 Agent の世代。変わったら旧世代の event を捨てる。
   */
  readonly agentGeneration: AgentGeneration;
  readonly event: EventKind;
  readonly payload?: unknown;
  readonly protocolMajor: number;
  /**
   * 単調増加の連番。10 進文字列で運ぶ。
   */
  readonly sequence: DecimalU64;
}

/**
 * 発行しうる event の全体。
 */
export type EventKind = "runner.observation.changed" | "node.observation.changed" | "operation.changed" | "log.appended" | "resync.required";

/**
 * 強制停止の要求 payload。確認 challenge を必須にする。
 */
export interface ForceStopRequest {
  /**
   * UI が提示した確認 challenge の応答。`null` なら受理しない。
   */
  readonly confirmation?: string | null;
  readonly expectedRevision: number;
  readonly nodeId: NodeId;
}

/**
 * handshake が不一致だった理由。
 *
 * 「接続できません」へ丸めない。本体・Agent・Guest は同じ互換表で管理するので
 * （`docs/03_ARCHITECTURE.md` §8）、どちら側を更新すべきかを利用者が判断できる
 * 必要がある。相手が古いのか新しいのかを分け、双方の major を payload に載せる。
 */
export type HandshakeRejection = {
  readonly expectedProtocolMajor: number;
  readonly peerProtocolMajor: number;
  readonly reason: "peer_too_old";
} | {
  readonly expectedProtocolMajor: number;
  readonly peerProtocolMajor: number;
  readonly reason: "peer_too_new";
};

/**
 * handshake の要求。呼び出し側の自己申告。
 *
 * `implementation_version` は診断とログの相関のためだけに使い、互換性の判定には
 * `protocol_major` だけを見る（`docs/03_ARCHITECTURE.md` §8）。
 */
export interface HandshakeRequest {
  readonly implementationVersion: string;
  readonly protocolMajor: number;
}

/**
 * handshake の応答。
 */
export interface HandshakeResult {
  readonly agentGeneration: AgentGeneration;
  readonly capabilities: readonly string[];
  readonly implementationVersion: string;
  readonly protocolMajor: number;
}

/**
 * ローカルのプロセス・Guest の観測状態。
 */
export type LocalRuntime = "not_provisioned" | "starting" | "running" | "stopping" | "stopped" | "lost" | "error" | "unknown";

/**
 * ワイヤー上を流れる 1 メッセージ。
 *
 * minor 追加との互換性のため payload 側は `Value` のまま受け取り、method ごとの
 * 具体型へは受理後に変換する。未知の `kind` は復号段階で拒否する。
 */
export type Message = Request & {
  readonly kind: "request";
} | Response & {
  readonly kind: "response";
} | Event & {
  readonly kind: "event";
};

/**
 * Agent が受け付ける method の全体。
 */
export type Method = "system.handshake" | "system.doctor" | "node.snapshot" | "node.start" | "node.stop" | "node.forceStop" | "runner.planCreate" | "runner.applyCreate" | "runner.planRemove" | "runner.applyRemove" | "operation.get" | "operation.cancel" | "auth.beginDevice" | "auth.cancel" | "auth.logout" | "credential.import" | "events.subscribe" | "logs.subscribe" | "logs.export" | "settings.plan" | "settings.apply";

/**
 * Node（物理 PC 1 台と所有ユーザー文脈の組）の ID。
 */
export type NodeId = string;

/**
 * `node.start` / `node.stop` の要求 payload。
 */
export interface NodeOperationRequest {
  /**
   * 直前に読んだ snapshot の revision。ずれていれば `REVISION_CONFLICT`。
   */
  readonly expectedRevision: number;
  readonly nodeId: NodeId;
}

/**
 * Node と配下 Runner の観測。秘密は含めない。
 */
export interface NodeSnapshot {
  readonly displayName: string;
  /**
   * Node 全体としての有効状態。片側だけ失敗していれば `Partial`。
   */
  readonly effective: EffectiveState;
  readonly nodeId: NodeId;
  readonly observedAt: Timestamp;
  /**
   * 楽観ロック用。`node.start` 等の `expectedRevision` と照合する。
   *
   * GitHub ID や event sequence と違い、10 進文字列にしない。§5 が文字列を課すのは
   * その 2 つで、revision は Agent が採番する小さな連番であり、§3 の例も数値で書く。
   */
  readonly revision: number;
  readonly runners: readonly RunnerObservation[];
}

/**
 * 最後に成功した観測（`verified_at`）からの鮮度。
 */
export type ObservationFreshness = "fresh" | "stale" | "never_observed";

/**
 * 要求を受け付けたという応答。完了ではない。
 */
export interface OperationAccepted {
  readonly accepted: boolean;
  /**
   * 同じ `requestId` の再送で既存 Operation へ収束した場合に `true`。
   *
   * 省略は「収束していない」を意味する。§3 の応答例がこの項目を持たないため、
   * 既定値を認めて例がそのまま復号できるようにする。
   */
  readonly deduplicated?: boolean;
  readonly operationId: OperationId;
}

/**
 * Operation 内の個別の失敗。
 */
export interface OperationFailure {
  readonly backendId?: BackendId | null;
  readonly code: ErrorCode;
  readonly messageKey: string;
}

/**
 * Operation の ID。
 */
export type OperationId = string;

/**
 * Operation の種別。
 */
export type OperationKind = "node_start" | "node_stop" | "node_force_stop" | "runner_create" | "runner_remove";

/**
 * Operation の進行段階。`docs/05_DOMAIN_STATE.md` §5 / §6 に対応する。
 */
export type OperationPhase = "requested" | "preflight" | "preparing" | "registering" | "starting" | "verifying" | "fresh_health_check" | "requires_force_confirmation" | "stop_signal" | "verify_exit" | "succeeded" | "failed" | "canceled" | "waiting_for_idle" | "partial";

/**
 * Operation の観測。
 */
export interface OperationSnapshot {
  /**
   * 段階ごとの結果。片側失敗を隠さない。
   */
  readonly failures: readonly OperationFailure[];
  readonly finishedAt?: Timestamp | null;
  readonly kind: OperationKind;
  readonly operationId: OperationId;
  readonly phase: OperationPhase;
  readonly startedAt: Timestamp;
  /**
   * 操作対象。Node か Runner の ID。
   */
  readonly targetId: string;
}

/**
 * GitHub が「受付可能」と認識しているかどうか。
 */
export type RemoteAvailability = "online_idle" | "online_busy" | "offline" | "unknown";

/**
 * GitHub 側に Runner 登録が存在するかどうか。`Online` とは別概念。
 */
export type RemotePresence = "unchecked" | "registered" | "not_found" | "unknown";

/**
 * UI から Agent への要求。
 */
export interface Request {
  /**
   * 固定 registry の method。未知の名前は復号で失敗する。
   */
  readonly method: Method;
  readonly payload?: unknown;
  readonly protocolMajor: number;
  /**
   * 冪等キー。同じ値の再送は同じ Operation へ収束させる。
   */
  readonly requestId: RequestId;
}

/**
 * 呼び出し側が採番する冪等キー。同じ値の再送を同じ Operation へ収束させる。
 */
export type RequestId = string;

/**
 * 要求に対する応答。
 */
export interface Response {
  readonly error?: ErrorPayload | null;
  readonly ok: boolean;
  readonly protocolMajor: number;
  readonly requestId: RequestId;
  /**
   * `ok=true` のときの結果。受付済みであって完了とは限らない。
   */
  readonly result?: unknown;
}

/**
 * ローカルで採番する Runner の ID。GitHub 側の ID とは別物。
 */
export type RunnerId = string;

/**
 * 1 つの Runner についての観測一式。
 */
export interface RunnerObservation {
  readonly backendId: BackendId;
  readonly desired: DesiredState;
  readonly effective: EffectiveState;
  /**
   * 鮮度が落ちたときに副表示する最後の既知値。
   */
  readonly lastKnownAvailability?: RemoteAvailability | null;
  readonly local: LocalRuntime;
  readonly localFreshness: ObservationFreshness;
  readonly remoteAvailability: RemoteAvailability;
  /**
   * 観測できなかった理由。丸めずに保持する。
   */
  readonly remoteErrorCode?: ErrorCode | null;
  readonly remoteFreshness: ObservationFreshness;
  readonly remotePresence: RemotePresence;
  /**
   * GitHub 側の Runner ID。登録完了まで `null`。
   */
  readonly remoteRunnerId?: DecimalU64 | null;
  readonly runnerId: RunnerId;
  readonly scopeId: ScopeId;
  /**
   * 最後に観測へ成功した時刻。一度も成功していなければ `null`。
   */
  readonly verifiedAt?: Timestamp | null;
}

/**
 * Scope（GitHub host と repo/org の組）の ID。
 */
export type ScopeId = string;

/**
 * Scope の種別。
 */
export type ScopeKind = "repo" | "org";

/**
 * UTC の RFC3339 時刻
 */
export type Timestamp = string;
