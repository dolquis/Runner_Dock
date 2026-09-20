//! エラー契約。`docs/10_IPC_DATA_MODEL.md` §8。
//!
//! 表示用メッセージと機械判定用 code を分離する。生 HTTP 応答や OS の
//! コマンドラインを `details` へ入れない。

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::ids::OperationId;

/// 機械判定用のエラー code。日本語訳を変えても動作が変わらないようにする。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ErrorCode {
    /// 認証が失効した。Runner が落ちたと断定しない。
    AuthExpired,
    PermissionOrPolicy,
    RateLimited,
    RunnerBusy,
    /// 観測が古い。成功へ丸めない。
    StatusStale,
    /// 管理対象外の path を操作しようとした。
    PathNotOwned,
    ProtocolMismatch,
    /// `expectedRevision` が現在値と食い違う。
    RevisionConflict,
    RequiresConfirmation,
    ChecksumMismatch,
    WslGuestUnreachable,
}

/// ワイヤー上のエラー payload。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ErrorPayload {
    pub code: ErrorCode,
    /// UI 側の翻訳キー。表示文字列そのものではない。
    pub message_key: String,
    pub retryable: bool,
    pub requires_confirmation: bool,
    pub operation_id: Option<OperationId>,
    /// 構造化した補足。秘密・生応答・コマンドラインを入れない。
    #[serde(default)]
    pub details: Map<String, Value>,
}

impl ErrorPayload {
    /// code から既定の翻訳キーと再試行可否を決めて payload を作る。
    #[must_use]
    pub fn new(code: ErrorCode) -> Self {
        Self {
            code,
            message_key: code.message_key().to_owned(),
            retryable: code.is_retryable(),
            requires_confirmation: code == ErrorCode::RequiresConfirmation,
            operation_id: None,
            details: Map::new(),
        }
    }

    #[must_use]
    pub fn with_operation(mut self, operation_id: OperationId) -> Self {
        self.operation_id = Some(operation_id);
        self
    }

    /// 構造化した補足を 1 件足す。値は呼び出し側が秘密を含まない形に整える。
    #[must_use]
    pub fn with_detail(mut self, key: &str, value: Value) -> Self {
        self.details.insert(key.to_owned(), value);
        self
    }
}

impl ErrorCode {
    /// UI 側の翻訳キー。
    #[must_use]
    pub const fn message_key(self) -> &'static str {
        match self {
            Self::AuthExpired => "errors.authExpired",
            Self::PermissionOrPolicy => "errors.permissionOrPolicy",
            Self::RateLimited => "errors.rateLimited",
            Self::RunnerBusy => "errors.runnerBusy",
            Self::StatusStale => "errors.statusStale",
            Self::PathNotOwned => "errors.pathNotOwned",
            Self::ProtocolMismatch => "errors.protocolMismatch",
            Self::RevisionConflict => "errors.revisionConflict",
            Self::RequiresConfirmation => "errors.requiresConfirmation",
            Self::ChecksumMismatch => "errors.checksumMismatch",
            Self::WslGuestUnreachable => "errors.wslGuestUnreachable",
        }
    }

    /// 同じ要求をそのまま再送してよいか。外部副作用の取消しとは別の判断。
    #[must_use]
    pub const fn is_retryable(self) -> bool {
        match self {
            Self::RateLimited
            | Self::RunnerBusy
            | Self::StatusStale
            | Self::WslGuestUnreachable => true,
            Self::AuthExpired
            | Self::PermissionOrPolicy
            | Self::PathNotOwned
            | Self::ProtocolMismatch
            | Self::RevisionConflict
            | Self::RequiresConfirmation
            | Self::ChecksumMismatch => false,
        }
    }
}
