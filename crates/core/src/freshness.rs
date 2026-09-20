//! 観測の鮮度。`docs/05_DOMAIN_STATE.md` §4 の表を正典とする。
//!
//! 観測間隔（次の観測を試みる周期）と stale 閾値（最後に成功した観測からこの時間を
//! 超えたら `Stale`）は別の数値で、同じ「鮮度」として扱わない。`Retry-After` 等で
//! 観測間隔が延びても閾値は延ばさない。

use runnerdock_protocol::dto::ObservationFreshness;

use crate::clock::UnixMillis;

/// 鮮度を測る対象。GUI の表示状態で remote の周期が変わる。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObservationTarget {
    /// ローカルのプロセス・Guest。
    Local,
    /// GitHub API、GUI 表示中。
    RemoteVisible,
    /// GitHub API、GUI 非表示。
    RemoteHidden,
}

impl ObservationTarget {
    /// 次の観測を試みる周期（ミリ秒）。
    #[must_use]
    pub const fn interval_millis(self) -> i64 {
        match self {
            Self::Local => 3_000,
            Self::RemoteVisible => 30_000,
            Self::RemoteHidden => 60_000,
        }
    }

    /// `verified_at` からこの時間を超えたら `Stale`（ミリ秒）。
    #[must_use]
    pub const fn stale_threshold_millis(self) -> i64 {
        match self {
            Self::Local => 10_000,
            Self::RemoteVisible => 90_000,
            Self::RemoteHidden => 180_000,
        }
    }
}

/// 鮮度を判定する。
///
/// `verified_at` は最後に観測へ**成功**した時刻。1 回の観測失敗では更新されないだけで、
/// それ自体が `Stale` を意味しない。閾値を超えて初めて `Stale` になる。
///
/// 経過時間が負のとき、つまり観測時刻が「現在」より後になっているときは `Stale` に
/// する。OS 時計が後方修正されると壁時計の差は当てにならず、何時間前の観測でも
/// 負の経過時間として `Fresh` に見えてしまうため。経過時間そのものは単調時計で
/// 測るのが本来で（`docs/10_IPC_DATA_MODEL.md` §5）、呼び出し側が単調な値を渡せる
/// ならそちらを使う。ここは壁時計しか無いときの安全側の既定である。
#[must_use]
pub fn evaluate(
    target: ObservationTarget,
    verified_at: Option<UnixMillis>,
    now: UnixMillis,
) -> ObservationFreshness {
    let Some(verified_at) = verified_at else {
        return ObservationFreshness::NeverObserved;
    };
    let age = now.elapsed_since(verified_at);
    if age < 0 || age > target.stale_threshold_millis() {
        ObservationFreshness::Stale
    } else {
        ObservationFreshness::Fresh
    }
}

/// 観測の記録。304 で「変化なし」と確認できた場合も `verified_at` を更新する。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VerifiedAt(Option<UnixMillis>);

impl VerifiedAt {
    /// 一度も観測していない状態。
    #[must_use]
    pub const fn never() -> Self {
        Self(None)
    }

    #[must_use]
    pub const fn at(instant: UnixMillis) -> Self {
        Self(Some(instant))
    }

    #[must_use]
    pub const fn get(self) -> Option<UnixMillis> {
        self.0
    }

    /// 観測に成功した。値が変わったかどうかは問わない。
    #[must_use]
    pub const fn observed(self, now: UnixMillis) -> Self {
        Self(Some(now))
    }

    /// 304 で状態が不変と確認できた。成功と同じく鮮度を更新する。
    #[must_use]
    pub const fn confirmed_unchanged(self, now: UnixMillis) -> Self {
        self.observed(now)
    }

    /// 観測に失敗した。鮮度は据え置き、値を成功へ丸めない。
    #[must_use]
    pub const fn failed(self) -> Self {
        self
    }
}
