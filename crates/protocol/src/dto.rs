//! 状態 DTO。`docs/05_DOMAIN_STATE.md` の希望状態・観測状態・有効状態に対応する。
//!
//! 不明値は `null` か明示 enum にし、`0` や `false` へ置換しない
//! （`docs/10_IPC_DATA_MODEL.md` §5）。秘密本体はここに載せない。
//!
//! DTO は未知フィールドを許容する。minor 追加で項目が増えた相手と接続しても、
//! 既知の項目だけを読んで動き続けるため（同 §9）。拒否するのは未知のメッセージ種別と
//! major 不一致で、これはフレーム復号が担う。

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
///
/// ワイヤー表現は `docs/10_IPC_DATA_MODEL.md` §6 の
/// `kind IN ('native_windows','wsl')` と一致させる。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum BackendKind {
    NativeWindows,
    Wsl,
}

impl BackendKind {
    /// 既知の Backend 種別の全体。
    pub const ALL: &'static [Self] = &[Self::NativeWindows, Self::Wsl];

    /// 永続化とワイヤーで使う識別子。表示名とは分ける（ADR-015）。
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::NativeWindows => "native_windows",
            Self::Wsl => "wsl",
        }
    }
}

/// [`BackendKind`] として解釈できない識別子。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseBackendKindError {
    /// 受け取った識別子。表示とログのために保つ。
    pub value: String,
}

impl std::fmt::Display for ParseBackendKindError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "未知の Backend 種別: {}", self.value)
    }
}

impl std::error::Error for ParseBackendKindError {}

impl std::str::FromStr for BackendKind {
    type Err = ParseBackendKindError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::ALL
            .iter()
            .copied()
            .find(|kind| kind.as_str() == s)
            .ok_or_else(|| ParseBackendKindError {
                value: s.to_owned(),
            })
    }
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
#[serde(rename_all = "camelCase")]
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
#[serde(rename_all = "camelCase")]
pub struct NodeSnapshot {
    pub node_id: NodeId,
    pub display_name: String,
    /// 楽観ロック用。`node.start` 等の `expectedRevision` と照合する。
    ///
    /// GitHub ID や event sequence と違い、10 進文字列にしない。§5 が文字列を課すのは
    /// その 2 つで、revision は Agent が採番する小さな連番であり、§3 の例も数値で書く。
    pub revision: u64,
    pub runners: Vec<RunnerObservation>,
    /// Node 全体としての有効状態。片側だけ失敗していれば `Partial`。
    pub effective: EffectiveState,
    pub observed_at: Timestamp,
}

/// 秘密本体を含まない資格情報の表示用ビュー。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
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
#[serde(rename_all = "camelCase")]
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
#[serde(rename_all = "camelCase")]
pub struct OperationFailure {
    pub backend_id: Option<BackendId>,
    pub code: ErrorCode,
    pub message_key: String,
}

/// `node.start` / `node.stop` の要求 payload。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct NodeOperationRequest {
    pub node_id: NodeId,
    /// 直前に読んだ snapshot の revision。ずれていれば `REVISION_CONFLICT`。
    pub expected_revision: u64,
}

/// 強制停止の要求 payload。確認 challenge を必須にする。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ForceStopRequest {
    pub node_id: NodeId,
    pub expected_revision: u64,
    /// UI が提示した確認 challenge の応答。`null` なら受理しない。
    pub confirmation: Option<String>,
}

/// 要求を受け付けたという応答。完了ではない。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct OperationAccepted {
    pub operation_id: OperationId,
    pub accepted: bool,
    /// 同じ `requestId` の再送で既存 Operation へ収束した場合に `true`。
    ///
    /// 省略は「収束していない」を意味する。§3 の応答例がこの項目を持たないため、
    /// 既定値を認めて例がそのまま復号できるようにする。
    #[serde(default)]
    pub deduplicated: bool,
}

/// handshake の要求。呼び出し側の自己申告。
///
/// `implementation_version` は診断とログの相関のためだけに使い、互換性の判定には
/// `protocol_major` だけを見る（`docs/03_ARCHITECTURE.md` §8）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct HandshakeRequest {
    pub protocol_major: u32,
    pub implementation_version: String,
}

/// handshake の応答。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct HandshakeResult {
    pub protocol_major: u32,
    pub implementation_version: String,
    pub agent_generation: crate::ids::AgentGeneration,
    pub capabilities: Vec<String>,
}

/// handshake が不一致だった理由。
///
/// 「接続できません」へ丸めない。本体・Agent・Guest は同じ互換表で管理するので
/// （`docs/03_ARCHITECTURE.md` §8）、どちら側を更新すべきかを利用者が判断できる
/// 必要がある。相手が古いのか新しいのかを分け、双方の major を payload に載せる。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
// 判別子は他の enum と同じ snake_case、フィールドは他の DTO と同じ camelCase。
#[serde(
    tag = "reason",
    rename_all = "snake_case",
    rename_all_fields = "camelCase"
)]
pub enum HandshakeRejection {
    /// 相手の major が古い。相手側の更新が要る。
    PeerTooOld {
        peer_protocol_major: u32,
        expected_protocol_major: u32,
    },
    /// 相手の major が新しい。こちら側の更新が要る。
    PeerTooNew {
        peer_protocol_major: u32,
        expected_protocol_major: u32,
    },
}

impl HandshakeRejection {
    /// 相手が申告した major を、この実装が受け入れられるか判定する。
    ///
    /// # Errors
    ///
    /// 一致しなければ、どちら側を更新すべきかを表す [`HandshakeRejection`] を返す。
    pub const fn evaluate(peer_protocol_major: u32) -> Result<(), Self> {
        let expected_protocol_major = crate::PROTOCOL_MAJOR;
        if peer_protocol_major == expected_protocol_major {
            return Ok(());
        }
        if peer_protocol_major < expected_protocol_major {
            return Err(Self::PeerTooOld {
                peer_protocol_major,
                expected_protocol_major,
            });
        }
        Err(Self::PeerTooNew {
            peer_protocol_major,
            expected_protocol_major,
        })
    }

    /// 更新すべきなのが相手側かどうか。UI の案内を分けるのに使う。
    #[must_use]
    pub const fn peer_must_update(self) -> bool {
        matches!(self, Self::PeerTooOld { .. })
    }
}

#[cfg(test)]
mod handshake_tests {
    use super::HandshakeRejection;
    use serde_json::json;

    #[test]
    fn a_matching_major_is_accepted() {
        assert_eq!(HandshakeRejection::evaluate(crate::PROTOCOL_MAJOR), Ok(()));
    }

    #[test]
    fn an_older_peer_is_told_to_update_itself() {
        let rejection = HandshakeRejection::evaluate(crate::PROTOCOL_MAJOR - 1).unwrap_err();

        assert!(rejection.peer_must_update());
        assert_eq!(
            serde_json::to_value(rejection).unwrap(),
            json!({
                "reason": "peer_too_old",
                "peerProtocolMajor": crate::PROTOCOL_MAJOR - 1,
                "expectedProtocolMajor": crate::PROTOCOL_MAJOR
            })
        );
    }

    #[test]
    fn a_newer_peer_means_this_side_must_update() {
        let rejection = HandshakeRejection::evaluate(crate::PROTOCOL_MAJOR + 1).unwrap_err();

        assert!(!rejection.peer_must_update());
        assert_eq!(
            serde_json::to_value(rejection).unwrap(),
            json!({
                "reason": "peer_too_new",
                "peerProtocolMajor": crate::PROTOCOL_MAJOR + 1,
                "expectedProtocolMajor": crate::PROTOCOL_MAJOR
            })
        );
    }

    #[test]
    fn a_rejection_never_collapses_into_a_single_reason() {
        // 「接続できません」へ丸めない。どちら側を更新すべきかが残る。
        let older = HandshakeRejection::evaluate(crate::PROTOCOL_MAJOR - 1).unwrap_err();
        let newer = HandshakeRejection::evaluate(crate::PROTOCOL_MAJOR + 1).unwrap_err();

        assert_ne!(older, newer);
        assert_ne!(older.peer_must_update(), newer.peer_must_update());
    }
}
