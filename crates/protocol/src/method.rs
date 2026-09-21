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

    /// ワイヤー上の method 名。`serde` の rename と同じ値を返す。
    ///
    /// 能力一覧やログで名前が要る箇所のために持つ。ここと `serde` の rename が
    /// ずれると、申告した能力と実際に受け付ける名前が食い違うので、試験で
    /// 直列化結果と突き合わせる。
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::SystemHandshake => "system.handshake",
            Self::SystemDoctor => "system.doctor",
            Self::NodeSnapshot => "node.snapshot",
            Self::NodeStart => "node.start",
            Self::NodeStop => "node.stop",
            Self::NodeForceStop => "node.forceStop",
            Self::RunnerPlanCreate => "runner.planCreate",
            Self::RunnerApplyCreate => "runner.applyCreate",
            Self::RunnerPlanRemove => "runner.planRemove",
            Self::RunnerApplyRemove => "runner.applyRemove",
            Self::OperationGet => "operation.get",
            Self::OperationCancel => "operation.cancel",
            Self::AuthBeginDevice => "auth.beginDevice",
            Self::AuthCancel => "auth.cancel",
            Self::AuthLogout => "auth.logout",
            Self::CredentialImport => "credential.import",
            Self::EventsSubscribe => "events.subscribe",
            Self::LogsSubscribe => "logs.subscribe",
            Self::LogsExport => "logs.export",
            Self::SettingsPlan => "settings.plan",
            Self::SettingsApply => "settings.apply",
        }
    }

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

#[cfg(test)]
mod tests {
    use super::Method;

    #[test]
    fn as_str_matches_the_wire_name() {
        for method in Method::ALL {
            let encoded = serde_json::to_value(method).unwrap();

            assert_eq!(encoded.as_str(), Some(method.as_str()));
        }
    }

    #[test]
    fn a_name_outside_the_registry_is_rejected() {
        assert!(serde_json::from_value::<Method>(serde_json::json!("exec")).is_err());
        assert!(serde_json::from_value::<Method>(serde_json::json!("node.Start")).is_err());
    }
}
