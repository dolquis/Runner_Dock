//! 時計の注入。core は OS の時計を直接読まない。
//!
//! 内部では UTC の Unix ミリ秒、ワイヤー上では RFC3339 文字列を使う
//! （`docs/10_IPC_DATA_MODEL.md` §5）。テストは [`FixedClock`] を挿し、同じ入力から
//! 必ず同じ観測を再現する。

use runnerdock_protocol::ids::Timestamp;

/// UTC の Unix ミリ秒。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct UnixMillis(pub i64);

impl UnixMillis {
    /// 2 点間の経過ミリ秒。`other` が未来なら負になる。
    #[must_use]
    pub const fn elapsed_since(self, other: Self) -> i64 {
        self.0 - other.0
    }

    /// ミリ秒を進めた時刻。
    #[must_use]
    pub const fn plus_millis(self, millis: i64) -> Self {
        Self(self.0 + millis)
    }

    /// RFC3339（UTC、ミリ秒精度）の文字列へ変換する。
    #[must_use]
    pub fn to_timestamp(self) -> Timestamp {
        Timestamp(format_rfc3339_utc(self.0))
    }
}

/// 単調・実時刻の供給源。
pub trait Clock {
    /// 現在の UTC 時刻。
    fn now(&self) -> UnixMillis;
}

/// テストと Mock 用の固定時計。`advance` で明示的にだけ進む。
#[derive(Debug, Clone)]
pub struct FixedClock {
    now: UnixMillis,
}

impl FixedClock {
    #[must_use]
    pub const fn new(now: UnixMillis) -> Self {
        Self { now }
    }

    /// 時計を進める。観測の鮮度試験はこれで境界をまたぐ。
    pub const fn advance_millis(&mut self, millis: i64) {
        self.now = self.now.plus_millis(millis);
    }

    pub const fn set(&mut self, now: UnixMillis) {
        self.now = now;
    }
}

impl Clock for FixedClock {
    fn now(&self) -> UnixMillis {
        self.now
    }
}

/// Unix ミリ秒を RFC3339（UTC、ミリ秒精度）へ整形する。
///
/// 日付換算は Howard Hinnant の civil-from-days と同じ手順で、1970-01-01 より前の
/// 値にも対応する。うるう秒は扱わない。
#[must_use]
pub fn format_rfc3339_utc(unix_millis: i64) -> String {
    let (days, millis_of_day) = {
        let day_millis = 86_400_000_i64;
        // 負の時刻でも「その日の 0 時からの経過」を正にするため、floor 除算を使う。
        let days = unix_millis.div_euclid(day_millis);
        let rest = unix_millis.rem_euclid(day_millis);
        (days, rest)
    };

    let (year, month, day) = civil_from_days(days);
    let millis = millis_of_day % 1_000;
    let total_seconds = millis_of_day / 1_000;
    let second = total_seconds % 60;
    let minute = (total_seconds / 60) % 60;
    let hour = total_seconds / 3_600;

    format!("{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}.{millis:03}Z")
}

/// 1970-01-01 からの日数を暦日へ変換する。
fn civil_from_days(days: i64) -> (i64, u32, u32) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let day_of_era = z.rem_euclid(146_097);
    let year_of_era =
        (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let year = year_of_era + era * 400;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let mp = (5 * day_of_year + 2) / 153;
    let day = u32::try_from(day_of_year - (153 * mp + 2) / 5 + 1).unwrap_or(1);
    let month = u32::try_from(if mp < 10 { mp + 3 } else { mp - 9 }).unwrap_or(1);
    let year = if month <= 2 { year + 1 } else { year };
    (year, month, day)
}
