//! GitHub REST 応答の `status` / `busy` を観測状態へ解釈する。
//!
//! `Unknown` は `false` ではない（`docs/05_DOMAIN_STATE.md` §9）。`busy` が欠落・
//! `null`・真偽値以外のときに Idle とみなす実装（`jq '.busy // true'` 相当の逆）を
//! 作らないため、値の形を型で区別してから判定する。
//!
//! この module は HTTP を呼ばない。呼び出し側が取得した JSON 断片だけを受け取る。

use runnerdock_protocol::dto::{RemoteAvailability, RemotePresence};
use runnerdock_protocol::error::ErrorCode;
use serde_json::Value;

/// `busy` フィールドの取りうる形。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BusyField {
    /// フィールド自体がない。
    Missing,
    /// `null`。
    Null,
    /// 真偽値。
    Bool(bool),
    /// 真偽値以外（数値・文字列など）。
    NotABool,
}

impl BusyField {
    /// JSON オブジェクトから取り出す。
    #[must_use]
    pub fn from_object(object: &Value) -> Self {
        match object.get("busy") {
            None => Self::Missing,
            Some(Value::Null) => Self::Null,
            Some(Value::Bool(value)) => Self::Bool(*value),
            Some(_) => Self::NotABool,
        }
    }

    /// 真偽値の完全一致で Idle 候補かどうかを返す。
    ///
    /// `Missing` / `Null` / `NotABool` は `true` にも `false` にも倒さない。
    #[must_use]
    pub const fn is_exactly_false(&self) -> bool {
        matches!(self, Self::Bool(false))
    }
}

/// `status` フィールドの取りうる形。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StatusField {
    Missing,
    Null,
    Online,
    Offline,
    /// 文字列だが既知の値ではない。最後の既知値を副表示するため原文を保つ。
    UnknownValue(String),
    /// 文字列ですらない。
    NotAString,
}

impl StatusField {
    /// JSON オブジェクトから取り出す。
    #[must_use]
    pub fn from_object(object: &Value) -> Self {
        match object.get("status") {
            None => Self::Missing,
            Some(Value::Null) => Self::Null,
            Some(Value::String(value)) => match value.as_str() {
                "online" => Self::Online,
                "offline" => Self::Offline,
                _ => Self::UnknownValue(value.clone()),
            },
            Some(_) => Self::NotAString,
        }
    }
}

/// 解釈の結果。丸めた値ではなく、判断の根拠も一緒に返す。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AvailabilityVerdict {
    pub availability: RemoteAvailability,
    /// 既知の enum に無い `status` の原文。UI の副表示に使う。
    pub unrecognized_status: Option<String>,
}

/// `status` と `busy` から [`RemoteAvailability`] を決める。
///
/// `online` かつ `busy == false`（真偽値の完全一致）のときだけ `OnlineIdle` とする。
/// `offline` は `busy` の形に関わらず `Offline`。それ以外はすべて `Unknown` で、
/// `Idle` にも成功にも丸めない。
#[must_use]
pub fn interpret_availability(status: &StatusField, busy: &BusyField) -> AvailabilityVerdict {
    let (availability, unrecognized_status) = match status {
        StatusField::Offline => (RemoteAvailability::Offline, None),
        StatusField::Online => {
            let availability = match busy {
                BusyField::Bool(false) => RemoteAvailability::OnlineIdle,
                BusyField::Bool(true) => RemoteAvailability::OnlineBusy,
                // busy 不明は Idle にしない。
                BusyField::Missing | BusyField::Null | BusyField::NotABool => {
                    RemoteAvailability::Unknown
                }
            };
            (availability, None)
        }
        StatusField::UnknownValue(raw) => (RemoteAvailability::Unknown, Some(raw.clone())),
        StatusField::Missing | StatusField::Null | StatusField::NotAString => {
            (RemoteAvailability::Unknown, None)
        }
    };

    AvailabilityVerdict {
        availability,
        unrecognized_status,
    }
}

/// GitHub から返った Runner オブジェクト 1 件を解釈する。
#[must_use]
pub fn interpret_runner_object(object: &Value) -> AvailabilityVerdict {
    interpret_availability(
        &StatusField::from_object(object),
        &BusyField::from_object(object),
    )
}

/// 観測そのものが失敗したときの扱い。
///
/// API のエラーや期限切れを `Offline` / `Idle` / 成功へ丸めず、`Unknown` と鮮度を
/// 保持する（AGENTS.md §5.1）。
#[must_use]
pub const fn availability_on_error(_code: ErrorCode) -> RemoteAvailability {
    RemoteAvailability::Unknown
}

/// 観測が失敗したときの登録有無の扱い。存在しないと断定しない。
#[must_use]
pub const fn presence_on_error(_code: ErrorCode) -> RemotePresence {
    RemotePresence::Unknown
}
