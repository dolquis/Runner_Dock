//! メッセージ封筒。`docs/10_IPC_DATA_MODEL.md` §3。

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::PROTOCOL_MAJOR;
use crate::error::ErrorPayload;
use crate::ids::{AgentGeneration, DecimalU64, RequestId};
use crate::method::Method;

/// ワイヤー上を流れる 1 メッセージ。
///
/// minor 追加との互換性のため payload 側は `Value` のまま受け取り、method ごとの
/// 具体型へは受理後に変換する。未知の `kind` は復号段階で拒否する。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum Message {
    Request(Request),
    Response(Response),
    Event(Event),
}

impl Message {
    /// このメッセージが宣言する protocol major。
    #[must_use]
    pub const fn protocol_major(&self) -> u32 {
        match self {
            Self::Request(m) => m.protocol_major,
            Self::Response(m) => m.protocol_major,
            Self::Event(m) => m.protocol_major,
        }
    }
}

/// UI から Agent への要求。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Request {
    pub protocol_major: u32,
    /// 冪等キー。同じ値の再送は同じ Operation へ収束させる。
    pub request_id: RequestId,
    /// 固定 registry の method。未知の名前は復号で失敗する。
    pub method: Method,
    #[serde(default)]
    pub payload: Value,
}

impl Request {
    #[must_use]
    pub fn new(request_id: RequestId, method: Method, payload: Value) -> Self {
        Self {
            protocol_major: PROTOCOL_MAJOR,
            request_id,
            method,
            payload,
        }
    }
}

/// 要求に対する応答。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Response {
    pub protocol_major: u32,
    pub request_id: RequestId,
    pub ok: bool,
    /// `ok=true` のときの結果。受付済みであって完了とは限らない。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<ErrorPayload>,
}

impl Response {
    #[must_use]
    pub fn ok(request_id: RequestId, result: Value) -> Self {
        Self {
            protocol_major: PROTOCOL_MAJOR,
            request_id,
            ok: true,
            result: Some(result),
            error: None,
        }
    }

    #[must_use]
    pub fn err(request_id: RequestId, error: ErrorPayload) -> Self {
        Self {
            protocol_major: PROTOCOL_MAJOR,
            request_id,
            ok: false,
            result: None,
            error: Some(error),
        }
    }
}

/// Agent が push する差分イベント。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Event {
    pub protocol_major: u32,
    /// 単調増加の連番。10 進文字列で運ぶ。
    pub sequence: DecimalU64,
    /// 発行元 Agent の世代。変わったら旧世代の event を捨てる。
    pub agent_generation: AgentGeneration,
    pub event: EventKind,
    #[serde(default)]
    pub payload: Value,
}

impl Event {
    #[must_use]
    pub fn new(
        sequence: DecimalU64,
        agent_generation: AgentGeneration,
        event: EventKind,
        payload: Value,
    ) -> Self {
        Self {
            protocol_major: PROTOCOL_MAJOR,
            sequence,
            agent_generation,
            event,
            payload,
        }
    }
}

/// 発行しうる event の全体。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
pub enum EventKind {
    #[serde(rename = "runner.observation.changed")]
    RunnerObservationChanged,
    #[serde(rename = "node.observation.changed")]
    NodeObservationChanged,
    #[serde(rename = "operation.changed")]
    OperationChanged,
    #[serde(rename = "log.appended")]
    LogAppended,
    /// 取りこぼしが起きたので snapshot を取り直せという通知。
    #[serde(rename = "resync.required")]
    ResyncRequired,
}
