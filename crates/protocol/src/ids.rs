//! 識別子と時刻の newtype。
//!
//! GitHub の数値 ID と event sequence は内部で `u64`、ワイヤー上では 10 進文字列に
//! する（`docs/10_IPC_DATA_MODEL.md` §5）。JavaScript の整数精度で壊れないように、
//! 境界を跨ぐ `u64` は必ず [`DecimalU64`] を通す。

use std::fmt;
use std::num::ParseIntError;
use std::str::FromStr;

use schemars::{JsonSchema, Schema, SchemaGenerator, json_schema};
use serde::de::{self, Deserializer, Visitor};
use serde::{Deserialize, Serialize, Serializer};

/// ワイヤー上は 10 進文字列、内部では `u64` として扱う整数。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct DecimalU64(u64);

impl DecimalU64 {
    #[must_use]
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }
}

impl From<u64> for DecimalU64 {
    fn from(value: u64) -> Self {
        Self(value)
    }
}

impl fmt::Display for DecimalU64 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl FromStr for DecimalU64 {
    type Err = ParseIntError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        s.parse::<u64>().map(Self)
    }
}

impl Serialize for DecimalU64 {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.0.to_string())
    }
}

impl<'de> Deserialize<'de> for DecimalU64 {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct DecimalVisitor;

        impl Visitor<'_> for DecimalVisitor {
            type Value = DecimalU64;

            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("10 進文字列で表した符号なし整数")
            }

            // 数値としてのフレームは受け付けない。JavaScript 側で精度が落ちた値を
            // 黙って取り込まないため、文字列だけを正とする。
            fn visit_str<E: de::Error>(self, v: &str) -> Result<Self::Value, E> {
                v.parse::<u64>()
                    .map(DecimalU64)
                    .map_err(|_| E::invalid_value(de::Unexpected::Str(v), &self))
            }
        }

        deserializer.deserialize_str(DecimalVisitor)
    }
}

impl JsonSchema for DecimalU64 {
    fn schema_name() -> std::borrow::Cow<'static, str> {
        "DecimalU64".into()
    }

    fn json_schema(_generator: &mut SchemaGenerator) -> Schema {
        json_schema!({
            "type": "string",
            "pattern": "^(0|[1-9][0-9]*)$",
            "description": "符号なし 64bit 整数を 10 進文字列で表したもの"
        })
    }
}

/// 文字列の newtype を、ワイヤー上は素の文字列として定義する。
macro_rules! string_id {
    ($(#[$meta:meta])* $name:ident) => {
        $(#[$meta])*
        #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, JsonSchema)]
        #[serde(transparent)]
        pub struct $name(pub String);

        impl $name {
            #[must_use]
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl From<&str> for $name {
            fn from(value: &str) -> Self {
                Self(value.to_owned())
            }
        }

        impl From<String> for $name {
            fn from(value: String) -> Self {
                Self(value)
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(&self.0)
            }
        }
    };
}

string_id!(
    /// Node（物理 PC 1 台と所有ユーザー文脈の組）の ID。
    NodeId
);
string_id!(
    /// Backend（`NativeWindows` または具体的な WSL 環境）の ID。
    BackendId
);
string_id!(
    /// Scope（GitHub host と repo/org の組）の ID。
    ScopeId
);
string_id!(
    /// ローカルで採番する Runner の ID。GitHub 側の ID とは別物。
    RunnerId
);
string_id!(
    /// Operation の ID。
    OperationId
);
string_id!(
    /// 呼び出し側が採番する冪等キー。同じ値の再送を同じ Operation へ収束させる。
    RequestId
);
string_id!(
    /// 秘密本体を含まない資格情報参照の ID。
    CredentialRefId
);
string_id!(
    /// Agent の世代。Agent が再起動すると変わり、旧世代の event を捨てるのに使う。
    AgentGeneration
);
string_id!(
    /// UTC の RFC3339 時刻。
    Timestamp
);
