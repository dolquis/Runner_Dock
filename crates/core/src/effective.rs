//! UI へ出す有効状態。`docs/05_DOMAIN_STATE.md` §4 の表を関数にしたもの。
//!
//! 観測が古い・未知・API 障害のときに緑（`Ready`）を出さないことがこの module の役割。

use runnerdock_protocol::dto::{
    EffectiveState, LocalRuntime, ObservationFreshness, RemoteAvailability, RemotePresence,
};
use runnerdock_protocol::error::ErrorCode;

/// 有効状態を決めるための入力一式。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EffectiveInput {
    pub local: LocalRuntime,
    pub local_freshness: ObservationFreshness,
    pub remote_presence: RemotePresence,
    pub remote_availability: RemoteAvailability,
    pub remote_freshness: ObservationFreshness,
    /// 観測できなかった理由。丸めずに渡す。
    pub remote_error_code: Option<ErrorCode>,
    /// 鮮度が落ちる前の最後の既知値。
    pub last_known_availability: Option<RemoteAvailability>,
}

/// 併記すべき矛盾。状態そのものを書き換えずに警告として持つ。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Conflict {
    /// GitHub は Busy と言っているのにローカルプロセスを見失っている。
    BusyWhileLocalLost,
    /// ローカルは停止しているのに GitHub からは Idle に見える。
    LocalStoppedWhileRemoteIdle,
}

/// 判定結果。副表示に使う値も一緒に返す。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EffectiveVerdict {
    pub state: EffectiveState,
    pub conflict: Option<Conflict>,
    /// `Unknown` のときに副表示する最後の既知値。
    pub last_known_availability: Option<RemoteAvailability>,
}

/// Runner 1 件の有効状態を決める。
#[must_use]
pub fn evaluate_runner(input: &EffectiveInput) -> EffectiveVerdict {
    let fallback = input.last_known_availability;

    // 観測手段そのものが失われている場合を先に見る。Runner が落ちたと断定しない。
    if matches!(input.remote_error_code, Some(ErrorCode::AuthExpired))
        && input.local == LocalRuntime::Running
    {
        return EffectiveVerdict {
            state: EffectiveState::MonitoringUnavailable,
            conflict: None,
            last_known_availability: fallback,
        };
    }

    // 古い観測・未知の値を現在の事実として扱わない。
    if input.remote_freshness != ObservationFreshness::Fresh
        || input.remote_availability == RemoteAvailability::Unknown
    {
        return EffectiveVerdict {
            state: EffectiveState::Unknown,
            conflict: None,
            last_known_availability: fallback,
        };
    }

    match (input.local, input.remote_availability) {
        (LocalRuntime::Lost, RemoteAvailability::OnlineBusy) => EffectiveVerdict {
            state: EffectiveState::Busy,
            conflict: Some(Conflict::BusyWhileLocalLost),
            last_known_availability: fallback,
        },
        (_, RemoteAvailability::OnlineBusy) => EffectiveVerdict {
            state: EffectiveState::Busy,
            conflict: None,
            last_known_availability: fallback,
        },
        (LocalRuntime::Running, RemoteAvailability::OnlineIdle) => EffectiveVerdict {
            state: EffectiveState::Ready,
            conflict: None,
            last_known_availability: fallback,
        },
        (LocalRuntime::Running, RemoteAvailability::Offline) => EffectiveVerdict {
            state: EffectiveState::Disconnected,
            conflict: None,
            last_known_availability: fallback,
        },
        (LocalRuntime::Stopped, RemoteAvailability::OnlineIdle) => EffectiveVerdict {
            // GitHub 反映待ちか PID 把握不足。緑にしない。
            state: EffectiveState::Reconciling,
            conflict: Some(Conflict::LocalStoppedWhileRemoteIdle),
            last_known_availability: fallback,
        },
        _ => EffectiveVerdict {
            state: EffectiveState::Unknown,
            conflict: None,
            last_known_availability: fallback,
        },
    }
}

/// 稼働として扱ってよい有効状態。
const fn is_healthy(state: EffectiveState) -> bool {
    matches!(state, EffectiveState::Ready | EffectiveState::Busy)
}

/// Node 全体の有効状態を配下 Runner から決める。
///
/// 片側だけ成功している Node を成功として出さない（`docs/05_DOMAIN_STATE.md` §4 の
/// `Partial`）。Runner が 1 件も無ければ `Unknown`。
#[must_use]
pub fn aggregate_node(runner_states: &[EffectiveState]) -> EffectiveState {
    let Some(&first) = runner_states.first() else {
        return EffectiveState::Unknown;
    };
    if runner_states.iter().all(|&state| state == first) {
        return first;
    }
    if runner_states.iter().all(|&state| is_healthy(state)) {
        // Ready と Busy の混在は正常な稼働。片方でも Ready なら Ready を出す。
        return if runner_states.contains(&EffectiveState::Ready) {
            EffectiveState::Ready
        } else {
            EffectiveState::Busy
        };
    }
    EffectiveState::Partial
}
