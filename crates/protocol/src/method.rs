//! 固定 method registry。`docs/10_IPC_DATA_MODEL.md` §4 の一覧と一致させる。
//!
//! 汎用 `exec` / `readFile` / `writeFile` / `sql` は public IPC として提供しない。
//! 列挙にない method 名は復号段階で拒否される。

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Agent が受け付ける method の全体。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
pub enum Method {
    #[serde(rename = "system.handshake")]
    SystemHandshake,
    #[serde(rename = "system.doctor")]
    SystemDoctor,
    #[serde(rename = "node.snapshot")]
    NodeSnapshot,
    #[serde(rename = "node.start")]
    NodeStart,
    #[serde(rename = "node.stop")]
    NodeStop,
    #[serde(rename = "node.forceStop")]
    NodeForceStop,
    #[serde(rename = "runner.planCreate")]
    RunnerPlanCreate,
    #[serde(rename = "runner.applyCreate")]
    RunnerApplyCreate,
    #[serde(rename = "runner.planRemove")]
    RunnerPlanRemove,
    #[serde(rename = "runner.applyRemove")]
    RunnerApplyRemove,
    #[serde(rename = "operation.get")]
    OperationGet,
    #[serde(rename = "operation.cancel")]
    OperationCancel,
    #[serde(rename = "auth.beginDevice")]
    AuthBeginDevice,
    #[serde(rename = "auth.cancel")]
    AuthCancel,
    #[serde(rename = "auth.logout")]
    AuthLogout,
    #[serde(rename = "credential.import")]
    CredentialImport,
    #[serde(rename = "events.subscribe")]
    EventsSubscribe,
    #[serde(rename = "logs.subscribe")]
    LogsSubscribe,
    #[serde(rename = "logs.export")]
    LogsExport,
    #[serde(rename = "settings.plan")]
    SettingsPlan,
    #[serde(rename = "settings.apply")]
    SettingsApply,
}

impl Method {
    /// registry の全 method。生成物の差分検査と拒否テストで使う。
    pub const ALL: &'static [Self] = &[
        Self::SystemHandshake,
        Self::SystemDoctor,
        Self::NodeSnapshot,
        Self::NodeStart,
        Self::NodeStop,
        Self::NodeForceStop,
        Self::RunnerPlanCreate,
        Self::RunnerApplyCreate,
        Self::RunnerPlanRemove,
        Self::RunnerApplyRemove,
        Self::OperationGet,
        Self::OperationCancel,
        Self::AuthBeginDevice,
        Self::AuthCancel,
        Self::AuthLogout,
        Self::CredentialImport,
        Self::EventsSubscribe,
        Self::LogsSubscribe,
        Self::LogsExport,
        Self::SettingsPlan,
        Self::SettingsApply,
    ];

    /// 外部からの副作用を伴うか。読取専用 method は再試行の判断が違う。
    #[must_use]
    pub const fn is_mutating(self) -> bool {
        matches!(
            self,
            Self::NodeStart
                | Self::NodeStop
                | Self::NodeForceStop
                | Self::RunnerApplyCreate
                | Self::RunnerApplyRemove
                | Self::OperationCancel
                | Self::AuthBeginDevice
                | Self::AuthCancel
                | Self::AuthLogout
                | Self::CredentialImport
                | Self::SettingsApply
                // 診断 ZIP をディスクへ書く。出力先確認を伴うので読取扱いにしない。
                | Self::LogsExport
        )
    }
}
