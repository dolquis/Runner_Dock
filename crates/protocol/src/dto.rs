//! 状態 DTO。`docs/05_DOMAIN_STATE.md` の希望状態・観測状態・有効状態に対応する。
//!
//! 不明値は `null` か明示 enum にし、`0` や `false` へ置換しない
//! （`docs/10_IPC_DATA_MODEL.md` §5）。秘密本体はここに載せない。

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::error::ErrorCode;
use crate::ids::{
    BackendId, CredentialRefId, DecimalU64, NodeId, OperationId, RunnerId, ScopeId, Timestamp,
};

/// 利用者の意図。進行中の操作とは別に保持する。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum DesiredState {
    Running,
    Stopped,
    Removed,
}

/// ローカルのプロセス・Guest の観測状態。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum LocalRuntime {
    NotProvisioned,
    Starting,
    Running,
    Stopping,
    Stopped,
    Lost,
    Error,
    Unknown,
}

/// GitHub 側に Runner 登録が存在するかどうか。`Online` とは別概念。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum RemotePresence {
    Unchecked,
    Registered,
    NotFound,
    Unknown,
}

/// GitHub が「受付可能」と認識しているかどうか。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum RemoteAvailability {
    OnlineIdle,
    OnlineBusy,
    Offline,
    Unknown,
}

/// 最後に成功した観測（`verified_at`）からの鮮度。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ObservationFreshness {
    Fresh,
    Stale,
    NeverObserved,
}

/// UI へ出す有効状態。`docs/05_DOMAIN_STATE.md` §4 の表に対応する。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum EffectiveState {
    /// 稼働しており受付可能という観測。予約済みという意味ではない。
    Ready,
    Busy,
    /// ローカルは動いているが GitHub から見えない。
    Disconnected,
    /// ローカル停止だが GitHub は Idle と言っている。緑にしない。
    Reconciling,
    /// API 認証失効等で観測自体ができない。Runner が落ちたと断定しない。
    MonitoringUnavailable,
    /// Node 内の Backend ごとに結果が割れている。
    Partial,
    Unknown,
}

/// Backend の種別。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum BackendKind {
    NativeWindows,
    Wsl,
}

/// Scope の種別。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ScopeKind {
    Repo,
    Org,
}

/// 1 つの Runner についての観測一式。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RunnerObservation {
    pub runner_id: RunnerId,
    pub backend_id: BackendId,
    pub scope_id: ScopeId,
    pub desired: DesiredState,
    pub local: LocalRuntime,
    pub local_freshness: ObservationFreshness,
    pub remote_presence: RemotePresence,
    pub remote_availability: RemoteAvailability,
    pub remote_freshness: ObservationFreshness,
    /// 観測できなかった理由。丸めずに保持する。
    pub remote_error_code: Option<ErrorCode>,
    /// GitHub 側の Runner ID。登録完了まで `null`。
    pub remote_runner_id: Option<DecimalU64>,
    /// 最後に観測へ成功した時刻。一度も成功していなければ `null`。
    pub verified_at: Option<Timestamp>,
    /// 鮮度が落ちたときに副表示する最後の既知値。
    pub last_known_availability: Option<RemoteAvailability>,
    pub effective: EffectiveState,
}

/// Node と配下 Runner の観測。秘密は含めない。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct NodeSnapshot {
    pub node_id: NodeId,
    pub display_name: String,
    /// 楽観ロック用。`node.start` 等の `expectedRevision` と照合する。
    pub revision: DecimalU64,
    pub runners: Vec<RunnerObservation>,
    /// Node 全体としての有効状態。片側だけ失敗していれば `Partial`。
    pub effective: EffectiveState,
    pub observed_at: Timestamp,
}

/// 秘密本体を含まない資格情報の表示用ビュー。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CredentialRefView {
    pub credential_ref_id: CredentialRefId,
    /// 用途。ローカル管理用と Workflow 監視用を混同しない。
    pub purpose: String,
    pub permission_summary: Vec<String>,
    pub expires_at: Option<Timestamp>,
}

/// Operation の種別。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum OperationKind {
    NodeStart,
    NodeStop,
    NodeForceStop,
    RunnerCreate,
    RunnerRemove,
}

/// Operation の進行段階。`docs/05_DOMAIN_STATE.md` §5 / §6 に対応する。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum OperationPhase {
    Requested,
    Preflight,
    Preparing,
    Registering,
    Starting,
    Verifying,
    FreshHealthCheck,
    /// 厳密な drain ではない。新しいジョブが割り当てられる競合は残る。
    WaitingForIdle,
    RequiresForceConfirmation,
    StopSignal,
    VerifyExit,
    Succeeded,
    /// 片側だけ成功した状態。成功側を自動削除しない。
    Partial,
    Failed,
    Canceled,
}

impl OperationPhase {
    /// これ以上進まない段階かどうか。
    #[must_use]
    pub const fn is_terminal(self) -> bool {
        matches!(
            self,
            Self::Succeeded | Self::Partial | Self::Failed | Self::Canceled
        )
    }
}

/// Operation の観測。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OperationSnapshot {
    pub operation_id: OperationId,
    pub kind: OperationKind,
    pub phase: OperationPhase,
    /// 操作対象。Node か Runner の ID。
    pub target_id: String,
    pub started_at: Timestamp,
    pub finished_at: Option<Timestamp>,
    /// 段階ごとの結果。片側失敗を隠さない。
    pub failures: Vec<OperationFailure>,
}

/// Operation 内の個別の失敗。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OperationFailure {
    pub backend_id: Option<BackendId>,
    pub code: ErrorCode,
    pub message_key: String,
}

/// `node.start` / `node.stop` の要求 payload。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct NodeOperationRequest {
    pub node_id: NodeId,
    /// 直前に読んだ snapshot の revision。ずれていれば `REVISION_CONFLICT`。
    pub expected_revision: DecimalU64,
}

/// 強制停止の要求 payload。確認 challenge を必須にする。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ForceStopRequest {
    pub node_id: NodeId,
    pub expected_revision: DecimalU64,
    /// UI が提示した確認 challenge の応答。`null` なら受理しない。
    pub confirmation: Option<String>,
}

/// 要求を受け付けたという応答。完了ではない。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OperationAccepted {
    pub operation_id: OperationId,
    pub accepted: bool,
    /// 同じ `requestId` の再送で既存 Operation へ収束した場合に `true`。
    pub deduplicated: bool,
}

/// handshake の応答。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct HandshakeResult {
    pub protocol_major: u32,
    pub implementation_version: String,
    pub agent_generation: crate::ids::AgentGeneration,
    pub capabilities: Vec<String>,
}
