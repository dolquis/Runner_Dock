//! 固定 seed の Mock Backend。
//!
//! `docs/15_DEVELOPER_GUIDE.md` §3 が求める fixture（Idle、Busy、Offline、Unknown、
//! Stale、Creating、PartialFailure、RateLimited、ReauthenticationRequired）を、
//! ネットワーク・PAT・GitHub App・実 Runner なしで再現する。
//!
//! remote の観測は、あらかじめ enum に畳んだ値ではなく GitHub REST 応答に相当する
//! JSON 断片として持ち、[`crate::remote`] の解釈を通す。Mock でも `busy` の欠落や
//! 型違いが本番と同じ経路で `Unknown` になる。
//!
//! Mock から実 GitHub へ自動的に落ちる経路は持たない。

use runnerdock_protocol::dto::{
    BackendKind, DesiredState, EffectiveState, LocalRuntime, NodeSnapshot, ObservationFreshness,
    OperationAccepted, OperationKind, OperationPhase, RemoteAvailability, RemotePresence,
    RunnerObservation,
};
use runnerdock_protocol::error::{ErrorCode, ErrorPayload};
use runnerdock_protocol::ids::{
    AgentGeneration, BackendId, DecimalU64, NodeId, RequestId, RunnerId, ScopeId,
};
use runnerdock_protocol::message::{Event, EventKind};
use serde_json::{Value, json};

use crate::clock::{Clock, FixedClock, UnixMillis};
use crate::effective::{self, EffectiveInput};
use crate::events::SequenceSource;
use crate::freshness::{self, ObservationTarget, VerifiedAt};
use crate::operation::{OperationRequest, OperationStore, StopReadiness};
use crate::remote;

/// Mock が再現する状況。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MockScenario {
    Idle,
    Busy,
    Offline,
    Unknown,
    Stale,
    Creating,
    PartialFailure,
    RateLimited,
    ReauthenticationRequired,
}

impl MockScenario {
    /// 再現すべき状況の全体。
    pub const ALL: &'static [Self] = &[
        Self::Idle,
        Self::Busy,
        Self::Offline,
        Self::Unknown,
        Self::Stale,
        Self::Creating,
        Self::PartialFailure,
        Self::RateLimited,
        Self::ReauthenticationRequired,
    ];
}

/// Mock が持つ Runner 1 件分の観測材料。
#[derive(Debug, Clone)]
struct MockRunner {
    runner_id: RunnerId,
    backend_id: BackendId,
    scope_id: ScopeId,
    desired: DesiredState,
    local: LocalRuntime,
    local_verified: VerifiedAt,
    /// GitHub REST の Runner オブジェクト相当。`None` は未観測。
    remote_object: Option<Value>,
    remote_verified: VerifiedAt,
    remote_error: Option<ErrorCode>,
    remote_runner_id: Option<DecimalU64>,
    last_known_availability: Option<RemoteAvailability>,
}

/// 固定 seed の Mock Backend。
///
/// 同じ seed と同じ操作列からは必ず同じ snapshot と event 列が出る。
#[derive(Debug)]
pub struct MockBackend {
    seed: u64,
    clock: FixedClock,
    node_id: NodeId,
    revision: u64,
    runners: Vec<MockRunner>,
    operations: OperationStore,
    sequence: SequenceSource,
    pending_events: Vec<Event>,
}

/// Mock の基準時刻（2026-09-20T00:00:00Z）。seed を変えても動かさない。
const BASE_INSTANT: UnixMillis = UnixMillis(1_789_862_400_000);

impl MockBackend {
    /// seed と状況から Mock を組み立てる。
    #[must_use]
    pub fn with_scenario(seed: u64, scenario: MockScenario) -> Self {
        let generation = AgentGeneration(format!("{:08x}", mix(seed)));
        let node_id = NodeId(format!("node-{:04x}", mix(seed ^ 0x6e6f_6465) & 0xffff));
        let clock = FixedClock::new(BASE_INSTANT);
        let runners = build_runners(seed, scenario, BASE_INSTANT);

        let mut backend = Self {
            seed,
            clock,
            node_id,
            revision: 1,
            runners,
            operations: OperationStore::new(),
            sequence: SequenceSource::new(generation, 1),
            pending_events: Vec::new(),
        };
        if scenario == MockScenario::Creating {
            backend.seed_creating_operation();
        }
        backend
    }

    #[must_use]
    pub const fn seed(&self) -> u64 {
        self.seed
    }

    #[must_use]
    pub const fn revision(&self) -> u64 {
        self.revision
    }

    #[must_use]
    pub fn node_id(&self) -> &NodeId {
        &self.node_id
    }

    #[must_use]
    pub fn agent_generation(&self) -> &AgentGeneration {
        self.sequence.generation()
    }

    /// 時計だけを進める。観測は更新しないので鮮度が落ちていく。
    pub const fn advance_millis(&mut self, millis: i64) {
        self.clock.advance_millis(millis);
    }

    /// 現在の観測から snapshot を組む。鮮度と有効状態はここで毎回計算する。
    #[must_use]
    pub fn snapshot(&self) -> NodeSnapshot {
        let now = self.clock.now();
        let runners: Vec<RunnerObservation> = self
            .runners
            .iter()
            .map(|runner| observe(runner, now))
            .collect();
        let states: Vec<EffectiveState> = runners.iter().map(|r| r.effective).collect();

        NodeSnapshot {
            node_id: self.node_id.clone(),
            display_name: "Mock Node".to_owned(),
            revision: self.revision,
            effective: effective::aggregate_node(&states),
            runners,
            observed_at: now.to_timestamp(),
        }
    }

    /// 起動を要求する。同じ `requestId` の再送は同じ Operation へ収束する。
    ///
    /// # Errors
    ///
    /// revision がずれていれば `REVISION_CONFLICT` を返す。
    pub fn request_start(
        &mut self,
        request_id: &RequestId,
        expected_revision: u64,
    ) -> Result<OperationAccepted, ErrorPayload> {
        self.submit(
            OperationKind::NodeStart,
            request_id,
            expected_revision,
            None,
        )
    }

    /// 停止を要求する。Busy と Unknown は `WaitingForIdle` で保留する。
    ///
    /// # Errors
    ///
    /// revision がずれていれば `REVISION_CONFLICT` を返す。
    pub fn request_stop(
        &mut self,
        request_id: &RequestId,
        expected_revision: u64,
    ) -> Result<OperationAccepted, ErrorPayload> {
        self.submit(OperationKind::NodeStop, request_id, expected_revision, None)
    }

    /// 強制停止を要求する。確認応答が無ければ受理しない。
    ///
    /// # Errors
    ///
    /// 確認応答が空なら `REQUIRES_CONFIRMATION`、revision がずれていれば
    /// `REVISION_CONFLICT` を返す。
    pub fn request_force_stop(
        &mut self,
        request_id: &RequestId,
        expected_revision: u64,
        confirmation: Option<&str>,
    ) -> Result<OperationAccepted, ErrorPayload> {
        self.submit(
            OperationKind::NodeForceStop,
            request_id,
            expected_revision,
            confirmation.map(str::to_owned),
        )
    }

    fn submit(
        &mut self,
        kind: OperationKind,
        request_id: &RequestId,
        expected_revision: u64,
        confirmation: Option<String>,
    ) -> Result<OperationAccepted, ErrorPayload> {
        let now = self.clock.now();
        let request = OperationRequest {
            request_id: request_id.clone(),
            kind,
            target_id: self.node_id.0.clone(),
            expected_revision,
            confirmation,
        };
        let readiness = self.stop_readiness(now);
        let accepted = self
            .operations
            .submit(&request, self.revision, readiness, now)?;

        if !accepted.deduplicated {
            // 受理した要求だけが desired state と revision を動かす。
            match kind {
                OperationKind::NodeStart => self.set_desired(DesiredState::Running),
                OperationKind::NodeStop | OperationKind::NodeForceStop => {
                    self.set_desired(DesiredState::Stopped);
                }
                OperationKind::RunnerCreate | OperationKind::RunnerRemove => {}
            }
            self.revision += 1;
            self.emit(EventKind::OperationChanged, operation_payload(&accepted));
        }
        Ok(accepted)
    }

    fn stop_readiness(&self, now: UnixMillis) -> StopReadiness {
        // Node 全体の停止可否は、最も保守的な Runner に合わせる。
        let observations: Vec<RunnerObservation> = self
            .runners
            .iter()
            .map(|runner| observe(runner, now))
            .collect();

        let all_idle = !observations.is_empty()
            && observations.iter().all(|o| {
                o.remote_availability == RemoteAvailability::OnlineIdle
                    && o.remote_freshness == ObservationFreshness::Fresh
            });

        StopReadiness {
            availability: if all_idle {
                RemoteAvailability::OnlineIdle
            } else {
                RemoteAvailability::Unknown
            },
            is_fresh: all_idle,
        }
    }

    fn set_desired(&mut self, desired: DesiredState) {
        for runner in &mut self.runners {
            runner.desired = desired;
        }
    }

    /// 受理済み Operation の段階を読む。
    #[must_use]
    pub fn operation_phase(&self, request_id: &RequestId) -> Option<OperationPhase> {
        self.operations
            .find_by_request(request_id)
            .map(|record| record.phase)
    }

    /// 溜まった event を取り出す。
    pub fn drain_events(&mut self) -> Vec<Event> {
        std::mem::take(&mut self.pending_events)
    }

    fn emit(&mut self, kind: EventKind, payload: Value) {
        let sequence = self.sequence.take();
        let generation = self.sequence.generation().clone();
        self.pending_events
            .push(Event::new(sequence, generation, kind, payload));
    }

    /// Creating 用に、進行中の登録 Operation を 1 件だけ仕込む。
    fn seed_creating_operation(&mut self) {
        let now = self.clock.now();
        let request = OperationRequest {
            request_id: RequestId(format!("req-create-{:04x}", mix(self.seed) & 0xffff)),
            kind: OperationKind::RunnerCreate,
            target_id: self.node_id.0.clone(),
            expected_revision: self.revision,
            confirmation: None,
        };
        let readiness = StopReadiness {
            availability: RemoteAvailability::Unknown,
            is_fresh: false,
        };
        if let Ok(accepted) = self
            .operations
            .submit(&request, self.revision, readiness, now)
        {
            self.operations
                .set_phase(&accepted.operation_id, OperationPhase::Registering, now);
        }
    }
}

/// 観測材料から 1 件の [`RunnerObservation`] を組む。
fn observe(runner: &MockRunner, now: UnixMillis) -> RunnerObservation {
    let local_freshness =
        freshness::evaluate(ObservationTarget::Local, runner.local_verified.get(), now);
    let remote_freshness = freshness::evaluate(
        ObservationTarget::RemoteVisible,
        runner.remote_verified.get(),
        now,
    );

    let (availability, presence, unrecognized) = match (&runner.remote_error, &runner.remote_object)
    {
        (Some(code), _) => (
            remote::availability_on_error(*code),
            remote::presence_on_error(*code),
            None,
        ),
        (None, Some(object)) => {
            let verdict = remote::interpret_runner_object(object);
            (
                verdict.availability,
                RemotePresence::Registered,
                verdict.unrecognized_status,
            )
        }
        (None, None) => (RemoteAvailability::Unknown, RemotePresence::Unchecked, None),
    };

    let verdict = effective::evaluate_runner(&EffectiveInput {
        local: runner.local,
        local_freshness,
        remote_availability: availability,
        remote_freshness,
        remote_error_code: runner.remote_error,
        last_known_availability: runner.last_known_availability,
    });

    // 未知の status 文字列も最後の既知値として副表示へ回せるように残す。
    let last_known = verdict
        .last_known_availability
        .or_else(|| unrecognized.as_ref().map(|_| RemoteAvailability::Unknown));

    RunnerObservation {
        runner_id: runner.runner_id.clone(),
        backend_id: runner.backend_id.clone(),
        scope_id: runner.scope_id.clone(),
        desired: runner.desired,
        local: runner.local,
        local_freshness,
        remote_presence: presence,
        remote_availability: availability,
        remote_freshness,
        remote_error_code: runner.remote_error,
        remote_runner_id: runner.remote_runner_id,
        verified_at: runner.remote_verified.get().map(UnixMillis::to_timestamp),
        last_known_availability: last_known,
        effective: verdict.state,
    }
}

fn operation_payload(accepted: &OperationAccepted) -> Value {
    json!({
        "operationId": accepted.operation_id.0,
        "accepted": accepted.accepted,
        "deduplicated": accepted.deduplicated,
    })
}

/// 状況ごとの Runner 2 件（Windows / WSL）を組む。
fn build_runners(seed: u64, scenario: MockScenario, base: UnixMillis) -> Vec<MockRunner> {
    let windows = base_runner(seed, BackendKind::NativeWindows, base);
    let wsl = base_runner(seed, BackendKind::Wsl, base);

    match scenario {
        MockScenario::Idle => vec![
            with_remote(windows, online(BackendKind::NativeWindows, false), base),
            with_remote(wsl, online(BackendKind::Wsl, false), base),
        ],
        MockScenario::Busy => vec![
            with_remote(windows, online(BackendKind::NativeWindows, true), base),
            with_remote(wsl, online(BackendKind::Wsl, true), base),
        ],
        MockScenario::Offline => vec![
            with_remote(windows, offline(BackendKind::NativeWindows), base),
            with_remote(wsl, offline(BackendKind::Wsl), base),
        ],
        MockScenario::Unknown => vec![
            // busy が無い応答。Idle にも Busy にもしない。
            with_remote(windows, json!({"id": 41, "status": "online"}), base),
            // busy が真偽値でない応答。
            with_remote(
                wsl,
                json!({"id": 42, "status": "online", "busy": "false"}),
                base,
            ),
        ],
        MockScenario::Stale => {
            // 観測間隔ではなく stale 閾値（remote 表示中は 90 秒）を超えた時刻。
            let old = base.plus_millis(-120_000);
            vec![
                stale(with_remote(
                    windows,
                    online(BackendKind::NativeWindows, false),
                    old,
                )),
                stale(with_remote(wsl, online(BackendKind::Wsl, false), old)),
            ]
        }
        MockScenario::Creating => vec![
            MockRunner {
                local: LocalRuntime::Starting,
                desired: DesiredState::Running,
                ..windows
            },
            MockRunner {
                local: LocalRuntime::NotProvisioned,
                desired: DesiredState::Running,
                ..wsl
            },
        ],
        MockScenario::PartialFailure => vec![
            with_remote(windows, online(BackendKind::NativeWindows, false), base),
            MockRunner {
                local: LocalRuntime::Error,
                remote_error: Some(ErrorCode::WslGuestUnreachable),
                ..wsl
            },
        ],
        MockScenario::RateLimited => vec![
            rate_limited(with_remote(
                windows,
                online(BackendKind::NativeWindows, false),
                base,
            )),
            rate_limited(with_remote(wsl, online(BackendKind::Wsl, false), base)),
        ],
        MockScenario::ReauthenticationRequired => vec![
            reauth(with_remote(
                windows,
                online(BackendKind::NativeWindows, false),
                base,
            )),
            reauth(with_remote(wsl, online(BackendKind::Wsl, false), base)),
        ],
    }
}

fn base_runner(seed: u64, kind: BackendKind, base: UnixMillis) -> MockRunner {
    let suffix = mix(seed ^ kind_salt(kind)) & 0xffff;
    let label = match kind {
        BackendKind::NativeWindows => "windows",
        BackendKind::Wsl => "wsl",
    };
    MockRunner {
        runner_id: RunnerId(format!("runner-{label}-{suffix:04x}")),
        backend_id: BackendId(format!("backend-{label}-{suffix:04x}")),
        scope_id: ScopeId(format!("scope-{:04x}", mix(seed ^ 0x7363_6f70) & 0xffff)),
        desired: DesiredState::Running,
        local: LocalRuntime::Running,
        local_verified: VerifiedAt::at(base),
        remote_object: None,
        remote_verified: VerifiedAt::never(),
        remote_error: None,
        remote_runner_id: None,
        last_known_availability: None,
    }
}

fn with_remote(runner: MockRunner, object: Value, verified_at: UnixMillis) -> MockRunner {
    let remote_runner_id = object
        .get("id")
        .and_then(Value::as_u64)
        .map(DecimalU64::new);
    MockRunner {
        remote_object: Some(object),
        remote_verified: VerifiedAt::at(verified_at),
        remote_runner_id,
        ..runner
    }
}

/// 観測は成功しているが古い。最後の既知値を副表示に残す。
fn stale(runner: MockRunner) -> MockRunner {
    MockRunner {
        last_known_availability: Some(RemoteAvailability::OnlineIdle),
        ..runner
    }
}

fn rate_limited(runner: MockRunner) -> MockRunner {
    MockRunner {
        remote_error: Some(ErrorCode::RateLimited),
        last_known_availability: Some(RemoteAvailability::OnlineIdle),
        ..runner
    }
}

fn reauth(runner: MockRunner) -> MockRunner {
    MockRunner {
        remote_error: Some(ErrorCode::AuthExpired),
        last_known_availability: Some(RemoteAvailability::OnlineIdle),
        ..runner
    }
}

/// 同じ scope 内で remote ID が衝突しないよう、Backend ごとに別の値を使う。
/// `docs/10_IPC_DATA_MODEL.md` §6 の `UNIQUE (scope_id, remote_runner_id)` と、
/// `docs/05_DOMAIN_STATE.md` §9 の remote identity に合わせる。
const fn remote_id_for(kind: BackendKind) -> u64 {
    match kind {
        BackendKind::NativeWindows => 41,
        BackendKind::Wsl => 42,
    }
}

fn online(kind: BackendKind, busy: bool) -> Value {
    json!({"id": remote_id_for(kind), "status": "online", "busy": busy})
}

fn offline(kind: BackendKind) -> Value {
    json!({"id": remote_id_for(kind), "status": "offline", "busy": false})
}

const fn kind_salt(kind: BackendKind) -> u64 {
    match kind {
        BackendKind::NativeWindows => 0x7769_6e00,
        BackendKind::Wsl => 0x7773_6c00,
    }
}

/// SplitMix64 の 1 段。seed から識別子を決定的に導く。
const fn mix(seed: u64) -> u64 {
    let mut z = seed.wrapping_add(0x9e37_79b9_7f4a_7c15);
    z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    z ^ (z >> 31)
}
