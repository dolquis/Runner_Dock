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
/// UTC の RFC3339 時刻（`docs/10_IPC_DATA_MODEL.md` §5）。
///
/// 境界で形式を検査する。相手が別の形式や現地時刻を送ってきたときに、黙って
/// 取り込んで鮮度計算を狂わせないため。
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(transparent)]
pub struct Timestamp(pub String);

impl Timestamp {
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// `YYYY-MM-DDThh:mm:ss[.fff]Z` として妥当か。
    ///
    /// 桁と区切りだけでなく、暦日と時刻の範囲も見る。`2026-99-99T99:99:99Z` のような
    /// 値を契約として受理すると、後段の日時パーサーや鮮度計算がそこで初めて失敗する。
    /// 時差付き表記と現地時刻も弾く。
    #[must_use]
    pub fn is_well_formed(text: &str) -> bool {
        let bytes = text.as_bytes();
        // 最短は "1970-01-01T00:00:00Z" の 20 バイト。
        if bytes.len() < 20 || !text.ends_with('Z') {
            return false;
        }
        let digits_at = |positions: &[usize]| {
            positions
                .iter()
                .all(|&i| bytes.get(i).is_some_and(u8::is_ascii_digit))
        };
        let separators_ok = bytes[4] == b'-'
            && bytes[7] == b'-'
            && bytes[10] == b'T'
            && bytes[13] == b':'
            && bytes[16] == b':';
        if !separators_ok || !digits_at(&[0, 1, 2, 3, 5, 6, 8, 9, 11, 12, 14, 15, 17, 18]) {
            return false;
        }

        // 秒の後ろは、そのまま `Z` か、小数点つきの小数秒のみ。
        let fraction_ok = match &text[19..text.len() - 1] {
            "" => true,
            fraction => {
                fraction.starts_with('.')
                    && fraction.len() > 1
                    && fraction[1..].bytes().all(|b| b.is_ascii_digit())
            }
        };
        if !fraction_ok {
            return false;
        }

        // 桁数は確認済みなので、ここでの parse は失敗しない。
        let field = |range: std::ops::Range<usize>| -> u32 {
            text.get(range)
                .and_then(|s| s.parse::<u32>().ok())
                .unwrap_or(u32::MAX)
        };
        let (year, month, day) = (field(0..4), field(5..7), field(8..10));
        let (hour, minute, second) = (field(11..13), field(14..16), field(17..19));

        if !(1..=12).contains(&month) || day < 1 || day > days_in_month(year, month) {
            return false;
        }
        // 秒は 60 も許す。RFC3339 はうるう秒の表記を認めている。
        hour <= 23 && minute <= 59 && second <= 60
    }
}

impl From<&str> for Timestamp {
    fn from(value: &str) -> Self {
        Self(value.to_owned())
    }
}

impl From<String> for Timestamp {
    fn from(value: String) -> Self {
        Self(value)
    }
}

impl fmt::Display for Timestamp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl<'de> Deserialize<'de> for Timestamp {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let text = String::deserialize(deserializer)?;
        if Self::is_well_formed(&text) {
            Ok(Self(text))
        } else {
            Err(de::Error::invalid_value(
                de::Unexpected::Str(&text),
                &"UTC の RFC3339 時刻（例 2026-09-20T00:00:00.000Z）",
            ))
        }
    }
}

impl JsonSchema for Timestamp {
    fn schema_name() -> std::borrow::Cow<'static, str> {
        "Timestamp".into()
    }

    fn json_schema(_generator: &mut SchemaGenerator) -> Schema {
        json_schema!({
            "type": "string",
            "format": "date-time",
            "pattern": "^\\d{4}-\\d{2}-\\d{2}T\\d{2}:\\d{2}:\\d{2}(\\.\\d+)?Z$",
            "description": "UTC の RFC3339 時刻"
        })
    }
}

/// その年月の日数。グレゴリオ暦のうるう年規則に従う。
const fn days_in_month(year: u32, month: u32) -> u32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => {
            if year.is_multiple_of(4) && (!year.is_multiple_of(100) || year.is_multiple_of(400)) {
                29
            } else {
                28
            }
        }
        _ => 0,
    }
}
