//! Backend の種別。
//!
//! Backend は Runner を動かす OS 上の実行場所を表す
//! （`docs/03_ARCHITECTURE.md` §3）。MVP では `async fn` を持つ trait の
//! `dyn` 化を前提にせず、列挙型と静的 dispatch を使う。

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

/// Runner を動かす実行場所の種別。
///
/// WSL が複数ある場合の distribution 名は Backend ID 側が持ち、この種別には
/// 含めない。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BackendKind {
    /// Windows 上で GitHub 公式 Runner を直接動かす。
    NativeWindows,
    /// WSL2 の distribution 内で GitHub 公式 Linux Runner を動かす。
    Wsl,
}

impl BackendKind {
    /// 設定ファイル・IPC・ログで使う安定した識別子。
    ///
    /// 表示名とは分離する。表示名の変更でワイヤー上の値が変わらないようにする。
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::NativeWindows => "native_windows",
            Self::Wsl => "wsl",
        }
    }

    /// 既知の種別をすべて返す。UI の選択肢と網羅テストで使う。
    #[must_use]
    pub const fn all() -> &'static [Self] {
        &[Self::NativeWindows, Self::Wsl]
    }
}

impl fmt::Display for BackendKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// 未知の Backend 識別子。
///
/// 未知の値を既定の Backend へ丸めると、別の実行場所で Runner を操作しうる。
/// 失敗として扱い、元の文字列を診断へ残す。
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("unknown backend kind: {input:?}")]
pub struct ParseBackendKindError {
    /// 解釈できなかった入力。
    pub input: String,
}

impl FromStr for BackendKind {
    type Err = ParseBackendKindError;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        match input {
            "native_windows" => Ok(Self::NativeWindows),
            "wsl" => Ok(Self::Wsl),
            other => Err(ParseBackendKindError {
                input: other.to_owned(),
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_kind_round_trips_through_its_identifier() {
        for kind in BackendKind::all() {
            assert_eq!(BackendKind::from_str(kind.as_str()), Ok(*kind));
        }
    }

    #[test]
    fn identifiers_are_distinct() {
        let mut seen = Vec::new();
        for kind in BackendKind::all() {
            assert!(!seen.contains(&kind.as_str()), "duplicate id: {kind}");
            seen.push(kind.as_str());
        }
    }

    #[test]
    fn returns_error_for_unknown_identifier_instead_of_defaulting() {
        let error = BackendKind::from_str("Wsl").expect_err("must not accept another casing");
        assert_eq!(error.input, "Wsl");
    }
}
