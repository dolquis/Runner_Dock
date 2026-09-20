//! core の受入相当の単体試験。
//!
//! 対応する受入試験は TC-003（busy の形）、TC-006（Local と GitHub の食い違い）、
//! TC-021（登録後の保存失敗と再試行）、TC-026（不正 frame・洪水・遅い購読者の
//! うちキュー部分）、TC-036（古い event・時計変更・304）。

use runnerdock_protocol::dto::{
    DesiredState, EffectiveState, LocalRuntime, ObservationFreshness, OperationKind,
    OperationPhase, RemoteAvailability, RemotePresence,
};
use runnerdock_protocol::error::{ErrorCode, ErrorPayload};
use runnerdock_protocol::ids::{AgentGeneration, DecimalU64, RequestId};
use runnerdock_protocol::message::{Event, EventKind};
use serde_json::json;

use crate::clock::{Clock, FixedClock, UnixMillis, format_rfc3339_utc};
use crate::effective::{self, Conflict, EffectiveInput};
use crate::events::{ApplyOutcome, EventApplier, PushOutcome, SubscriberQueue};
use crate::freshness::{self, ObservationTarget, VerifiedAt};
use crate::mock::{MockBackend, MockScenario};
use crate::operation::{OperationRequest, OperationStore, StopReadiness};
use crate::remote::{self, BusyField, StatusField};

const T0: UnixMillis = UnixMillis(1_789_862_400_000);

// ---- TC-003: busy と status の形ごとの解釈 ----------------------------------

fn availability_of(object: &serde_json::Value) -> RemoteAvailability {
    remote::interpret_runner_object(object).availability
}

#[test]
fn only_an_exact_false_busy_makes_a_runner_idle() {
    assert_eq!(
        availability_of(&json!({"status": "online", "busy": false})),
        RemoteAvailability::OnlineIdle
    );
}

#[test]
fn a_true_busy_makes_a_runner_busy() {
    assert_eq!(
        availability_of(&json!({"status": "online", "busy": true})),
        RemoteAvailability::OnlineBusy
    );
}

#[test]
fn a_null_busy_is_unknown_not_idle() {
    assert_eq!(
        availability_of(&json!({"status": "online", "busy": null})),
        RemoteAvailability::Unknown
    );
}

#[test]
fn a_missing_busy_is_unknown_not_idle() {
    assert_eq!(
        availability_of(&json!({"status": "online"})),
        RemoteAvailability::Unknown
    );
}

#[test]
fn a_string_busy_is_unknown_not_idle() {
    // "false" という文字列を偽として読まない。
    assert_eq!(
        availability_of(&json!({"status": "online", "busy": "false"})),
        RemoteAvailability::Unknown
    );
}

#[test]
fn an_unknown_status_is_unknown_and_keeps_the_raw_value() {
    let verdict = remote::interpret_runner_object(&json!({"status": "draining", "busy": false}));

    assert_eq!(verdict.availability, RemoteAvailability::Unknown);
    assert_eq!(verdict.unrecognized_status.as_deref(), Some("draining"));
}

#[test]
fn an_offline_status_is_offline_whatever_busy_says() {
    assert_eq!(
        availability_of(&json!({"status": "offline", "busy": true})),
        RemoteAvailability::Offline
    );
    assert_eq!(
        availability_of(&json!({"status": "offline"})),
        RemoteAvailability::Offline
    );
}

#[test]
fn busy_field_distinguishes_every_shape() {
    assert_eq!(
        BusyField::from_object(&json!({"busy": false})),
        BusyField::Bool(false)
    );
    assert_eq!(
        BusyField::from_object(&json!({"busy": null})),
        BusyField::Null
    );
    assert_eq!(BusyField::from_object(&json!({})), BusyField::Missing);
    assert_eq!(
        BusyField::from_object(&json!({"busy": 0})),
        BusyField::NotABool
    );

    // 「偽と等しい」のは真偽値の false だけ。
    assert!(BusyField::Bool(false).is_exactly_false());
    for other in [BusyField::Null, BusyField::Missing, BusyField::NotABool] {
        assert!(!other.is_exactly_false(), "{other:?} を偽として扱っている");
    }
}

#[test]
fn an_api_error_never_rounds_to_offline_or_idle() {
    for code in [
        ErrorCode::AuthExpired,
        ErrorCode::RateLimited,
        ErrorCode::PermissionOrPolicy,
    ] {
        assert_eq!(
            remote::availability_on_error(code),
            RemoteAvailability::Unknown
        );
        assert_eq!(remote::presence_on_error(code), RemotePresence::Unknown);
    }
}

#[test]
fn a_non_string_status_is_unknown() {
    assert_eq!(
        StatusField::from_object(&json!({"status": 1})),
        StatusField::NotAString
    );
    assert_eq!(
        remote::interpret_availability(&StatusField::NotAString, &BusyField::Bool(false))
            .availability,
        RemoteAvailability::Unknown
    );
}

// ---- TC-006: Local と GitHub の食い違い -------------------------------------

fn input(
    local: LocalRuntime,
    availability: RemoteAvailability,
    freshness: ObservationFreshness,
) -> EffectiveInput {
    EffectiveInput {
        local,
        local_freshness: ObservationFreshness::Fresh,
        remote_presence: RemotePresence::Registered,
        remote_availability: availability,
        remote_freshness: freshness,
        remote_error_code: None,
        last_known_availability: None,
    }
}

#[test]
fn a_running_runner_seen_idle_is_ready() {
    let verdict = effective::evaluate_runner(&input(
        LocalRuntime::Running,
        RemoteAvailability::OnlineIdle,
        ObservationFreshness::Fresh,
    ));

    assert_eq!(verdict.state, EffectiveState::Ready);
}

#[test]
fn a_running_runner_seen_offline_is_disconnected_not_ready() {
    let verdict = effective::evaluate_runner(&input(
        LocalRuntime::Running,
        RemoteAvailability::Offline,
        ObservationFreshness::Fresh,
    ));

    assert_eq!(verdict.state, EffectiveState::Disconnected);
}

#[test]
fn a_stopped_runner_seen_idle_is_reconciling_not_ready() {
    let verdict = effective::evaluate_runner(&input(
        LocalRuntime::Stopped,
        RemoteAvailability::OnlineIdle,
        ObservationFreshness::Fresh,
    ));

    assert_eq!(verdict.state, EffectiveState::Reconciling);
    assert_eq!(
        verdict.conflict,
        Some(Conflict::LocalStoppedWhileRemoteIdle)
    );
}

#[test]
fn a_busy_runner_whose_local_process_is_lost_is_busy_with_a_conflict() {
    let verdict = effective::evaluate_runner(&input(
        LocalRuntime::Lost,
        RemoteAvailability::OnlineBusy,
        ObservationFreshness::Fresh,
    ));

    assert_eq!(verdict.state, EffectiveState::Busy);
    assert_eq!(verdict.conflict, Some(Conflict::BusyWhileLocalLost));
}

#[test]
fn a_stale_observation_is_unknown_even_when_the_last_value_was_idle() {
    let mut raw = input(
        LocalRuntime::Running,
        RemoteAvailability::OnlineIdle,
        ObservationFreshness::Stale,
    );
    raw.last_known_availability = Some(RemoteAvailability::OnlineIdle);

    let verdict = effective::evaluate_runner(&raw);

    assert_eq!(verdict.state, EffectiveState::Unknown);
    assert_eq!(
        verdict.last_known_availability,
        Some(RemoteAvailability::OnlineIdle)
    );
}

#[test]
fn a_never_observed_runner_is_unknown() {
    let verdict = effective::evaluate_runner(&input(
        LocalRuntime::Running,
        RemoteAvailability::Unknown,
        ObservationFreshness::NeverObserved,
    ));

    assert_eq!(verdict.state, EffectiveState::Unknown);
}

#[test]
fn an_expired_credential_reports_monitoring_unavailable_not_a_dead_runner() {
    let mut raw = input(
        LocalRuntime::Running,
        RemoteAvailability::Unknown,
        ObservationFreshness::Fresh,
    );
    raw.remote_error_code = Some(ErrorCode::AuthExpired);

    let verdict = effective::evaluate_runner(&raw);

    assert_eq!(verdict.state, EffectiveState::MonitoringUnavailable);
}

#[test]
fn a_node_with_one_healthy_and_one_failing_backend_is_partial() {
    let state = effective::aggregate_node(&[EffectiveState::Ready, EffectiveState::Unknown]);

    assert_eq!(state, EffectiveState::Partial);
}

#[test]
fn a_node_whose_backends_are_ready_and_busy_is_not_partial() {
    let state = effective::aggregate_node(&[EffectiveState::Ready, EffectiveState::Busy]);

    assert_eq!(state, EffectiveState::Ready);
}

#[test]
fn a_node_without_runners_is_unknown() {
    assert_eq!(effective::aggregate_node(&[]), EffectiveState::Unknown);
}

// ---- 鮮度 --------------------------------------------------------------------

#[test]
fn an_observation_within_the_threshold_is_fresh() {
    let verified = VerifiedAt::at(T0);

    let result = freshness::evaluate(
        ObservationTarget::RemoteVisible,
        verified.get(),
        T0.plus_millis(90_000),
    );

    assert_eq!(result, ObservationFreshness::Fresh);
}

#[test]
fn an_observation_past_the_threshold_is_stale() {
    let result = freshness::evaluate(
        ObservationTarget::RemoteVisible,
        VerifiedAt::at(T0).get(),
        T0.plus_millis(90_001),
    );

    assert_eq!(result, ObservationFreshness::Stale);
}

#[test]
fn a_single_failed_observation_does_not_make_a_runner_stale() {
    // 観測間隔（30 秒）を 1 回落としただけでは閾値（90 秒）に届かない。
    let verified = VerifiedAt::at(T0).failed();

    let result = freshness::evaluate(
        ObservationTarget::RemoteVisible,
        verified.get(),
        T0.plus_millis(60_000),
    );

    assert_eq!(result, ObservationFreshness::Fresh);
}

#[test]
fn a_304_response_refreshes_the_verified_time() {
    let verified = VerifiedAt::at(T0).confirmed_unchanged(T0.plus_millis(80_000));

    let result = freshness::evaluate(
        ObservationTarget::RemoteVisible,
        verified.get(),
        T0.plus_millis(120_000),
    );

    assert_eq!(result, ObservationFreshness::Fresh);
}

#[test]
fn the_stale_threshold_is_about_three_observation_intervals() {
    // 1 回の観測失敗だけで Stale へ落とさない方針。厳密な 3 倍ではなく、
    // 3 倍以上 4 倍未満であることを確かめる（Local は 3 秒 / 10 秒）。
    for target in [
        ObservationTarget::Local,
        ObservationTarget::RemoteVisible,
        ObservationTarget::RemoteHidden,
    ] {
        let interval = target.interval_millis();
        let threshold = target.stale_threshold_millis();

        assert!(
            threshold >= interval * 3 && threshold < interval * 4,
            "{target:?} の閾値 {threshold} が観測間隔 {interval} のおよそ 3 倍でない"
        );
    }
}

#[test]
fn a_never_observed_target_is_reported_as_such() {
    assert_eq!(
        freshness::evaluate(ObservationTarget::Local, VerifiedAt::never().get(), T0),
        ObservationFreshness::NeverObserved
    );
}

// ---- TC-021: 冪等性・revision・停止の保留 -----------------------------------

fn request(kind: OperationKind, id: &str, revision: u64) -> OperationRequest {
    OperationRequest {
        request_id: RequestId::from(id),
        kind,
        target_id: "node-home".to_owned(),
        expected_revision: revision,
        confirmation: None,
    }
}

const IDLE: StopReadiness = StopReadiness {
    availability: RemoteAvailability::OnlineIdle,
    is_fresh: true,
};

#[test]
fn resending_the_same_request_id_converges_on_one_operation() {
    let mut store = OperationStore::new();
    let req = request(OperationKind::NodeStart, "req-1", 1);

    let first = store.submit(&req, 1, IDLE, T0).unwrap();
    let second = store.submit(&req, 1, IDLE, T0).unwrap();

    assert!(!first.deduplicated);
    assert!(second.deduplicated);
    assert_eq!(first.operation_id, second.operation_id);
}

#[test]
fn a_retry_after_a_failed_local_save_finds_the_recorded_remote_id() {
    // 登録の HTTP 応答は成功したがローカル保存に失敗した、という想定。
    let mut store = OperationStore::new();
    let req = request(OperationKind::RunnerCreate, "req-create", 1);
    let accepted = store.submit(&req, 1, IDLE, T0).unwrap();
    store.record_registration(&accepted.operation_id, DecimalU64::new(41));

    let retry = store.submit(&req, 1, IDLE, T0.plus_millis(5_000)).unwrap();

    assert!(retry.deduplicated);
    let record = store.find_by_request(&req.request_id).unwrap();
    // 再登録ではなく照合へ回すための材料が残っている。
    assert_eq!(record.registered_remote_id, Some(DecimalU64::new(41)));
    assert_eq!(record.phase, OperationPhase::Verifying);
}

#[test]
fn a_stale_revision_is_rejected_as_a_conflict() {
    let mut store = OperationStore::new();

    let error = store
        .submit(&request(OperationKind::NodeStart, "req-1", 2), 3, IDLE, T0)
        .unwrap_err();

    assert_eq!(error.code, ErrorCode::RevisionConflict);
    assert!(!error.retryable);
}

#[test]
fn a_resend_is_honoured_even_after_the_revision_moved_on() {
    // 受理済みの要求を、revision が進んだだけで二重受付にも失敗にもしない。
    let mut store = OperationStore::new();
    let req = request(OperationKind::NodeStart, "req-1", 1);
    let first = store.submit(&req, 1, IDLE, T0).unwrap();

    let resent = store.submit(&req, 2, IDLE, T0).unwrap();

    assert_eq!(resent.operation_id, first.operation_id);
    assert!(resent.deduplicated);
}

#[test]
fn stopping_a_busy_runner_waits_for_idle() {
    let mut store = OperationStore::new();
    let busy = StopReadiness {
        availability: RemoteAvailability::OnlineBusy,
        is_fresh: true,
    };

    let accepted = store
        .submit(
            &request(OperationKind::NodeStop, "req-stop", 1),
            1,
            busy,
            T0,
        )
        .unwrap();

    assert_eq!(
        store.get(&accepted.operation_id).unwrap().phase,
        OperationPhase::WaitingForIdle
    );
}

#[test]
fn stopping_a_runner_whose_state_is_unknown_also_waits() {
    let mut store = OperationStore::new();
    let unknown = StopReadiness {
        availability: RemoteAvailability::Unknown,
        is_fresh: true,
    };

    let accepted = store
        .submit(
            &request(OperationKind::NodeStop, "req-stop", 1),
            1,
            unknown,
            T0,
        )
        .unwrap();

    assert_eq!(
        store.get(&accepted.operation_id).unwrap().phase,
        OperationPhase::WaitingForIdle
    );
}

#[test]
fn a_stale_idle_observation_does_not_authorise_an_immediate_stop() {
    let mut store = OperationStore::new();
    let stale_idle = StopReadiness {
        availability: RemoteAvailability::OnlineIdle,
        is_fresh: false,
    };

    let accepted = store
        .submit(
            &request(OperationKind::NodeStop, "req-stop", 1),
            1,
            stale_idle,
            T0,
        )
        .unwrap();

    assert_eq!(
        store.get(&accepted.operation_id).unwrap().phase,
        OperationPhase::WaitingForIdle
    );
}

#[test]
fn stopping_a_confirmed_idle_runner_sends_the_stop_signal() {
    let mut store = OperationStore::new();

    let accepted = store
        .submit(
            &request(OperationKind::NodeStop, "req-stop", 1),
            1,
            IDLE,
            T0,
        )
        .unwrap();

    assert_eq!(
        store.get(&accepted.operation_id).unwrap().phase,
        OperationPhase::StopSignal
    );
}

#[test]
fn a_force_stop_without_confirmation_is_refused() {
    let mut store = OperationStore::new();

    let error = store
        .submit(
            &request(OperationKind::NodeForceStop, "req-force", 1),
            1,
            IDLE,
            T0,
        )
        .unwrap_err();

    assert_eq!(error.code, ErrorCode::RequiresConfirmation);
    assert!(error.requires_confirmation);
}

#[test]
fn a_force_stop_with_a_blank_confirmation_is_refused() {
    let mut store = OperationStore::new();
    let mut req = request(OperationKind::NodeForceStop, "req-force", 1);
    req.confirmation = Some("   ".to_owned());

    let error = store.submit(&req, 1, IDLE, T0).unwrap_err();

    assert_eq!(error.code, ErrorCode::RequiresConfirmation);
}

#[test]
fn a_confirmed_force_stop_is_accepted() {
    let mut store = OperationStore::new();
    let mut req = request(OperationKind::NodeForceStop, "req-force", 1);
    req.confirmation = Some("runner-windows".to_owned());

    let accepted = store.submit(&req, 1, IDLE, T0).unwrap();

    assert_eq!(
        store.get(&accepted.operation_id).unwrap().phase,
        OperationPhase::StopSignal
    );
}

#[test]
fn an_error_payload_carries_a_separate_code_and_message_key() {
    let payload = ErrorPayload::new(ErrorCode::WslGuestUnreachable)
        .with_detail("backendId", json!("backend-ubuntu"));

    assert_eq!(payload.code, ErrorCode::WslGuestUnreachable);
    assert_eq!(payload.message_key, "errors.wslGuestUnreachable");
    assert!(payload.retryable);
    // 表示文字列そのものは載せない。
    assert!(!payload.message_key.contains(' '));
}

// ---- TC-036: 世代・連番・時計 ------------------------------------------------

fn event_at(sequence: u64, generation: &str) -> Event {
    Event::new(
        DecimalU64::new(sequence),
        AgentGeneration::from(generation),
        EventKind::RunnerObservationChanged,
        json!({}),
    )
}

#[test]
fn events_from_a_previous_agent_generation_are_dropped() {
    let mut applier = EventApplier::new(AgentGeneration::from("gen-2"), 10);

    let outcome = applier.apply(&event_at(10, "gen-1"));

    assert!(matches!(
        outcome,
        ApplyOutcome::DroppedForeignGeneration { .. }
    ));
    // 捨てた event で進めない。
    assert_eq!(applier.next_sequence(), 10);
}

#[test]
fn an_event_older_than_the_applied_state_is_dropped() {
    let mut applier = EventApplier::new(AgentGeneration::from("gen-1"), 10);

    let outcome = applier.apply(&event_at(9, "gen-1"));

    assert_eq!(
        outcome,
        ApplyOutcome::DroppedStale {
            sequence: 9,
            expected: 10
        }
    );
}

#[test]
fn a_gap_in_the_sequence_requires_a_resync() {
    let mut applier = EventApplier::new(AgentGeneration::from("gen-1"), 10);

    let outcome = applier.apply(&event_at(12, "gen-1"));

    assert_eq!(
        outcome,
        ApplyOutcome::ResyncRequired {
            gap_from: 10,
            received: 12
        }
    );
    // snapshot を取り直すまで進めない。
    assert_eq!(applier.next_sequence(), 10);
}

#[test]
fn consecutive_events_are_applied_in_order() {
    let mut applier = EventApplier::new(AgentGeneration::from("gen-1"), 1);

    for sequence in 1..=3 {
        assert_eq!(
            applier.apply(&event_at(sequence, "gen-1")),
            ApplyOutcome::Applied
        );
    }

    assert_eq!(applier.next_sequence(), 4);
}

#[test]
fn ordering_follows_the_sequence_even_when_the_clock_moves_backwards() {
    // 時計が戻っても連番は戻らない。順序判断に時刻を使わない。
    let mut applier = EventApplier::new(AgentGeneration::from("gen-1"), 1);
    let mut clock = FixedClock::new(T0);
    assert_eq!(applier.apply(&event_at(1, "gen-1")), ApplyOutcome::Applied);

    clock.advance_millis(-60_000);

    assert_eq!(applier.apply(&event_at(2, "gen-1")), ApplyOutcome::Applied);
    assert_eq!(clock.now(), T0.plus_millis(-60_000));
}

#[test]
fn rebinding_after_a_restart_accepts_the_new_generation() {
    let mut applier = EventApplier::new(AgentGeneration::from("gen-1"), 10);

    applier.rebind(AgentGeneration::from("gen-2"), 1);

    assert_eq!(applier.apply(&event_at(1, "gen-2")), ApplyOutcome::Applied);
}

// ---- TC-026: 遅い購読者 ------------------------------------------------------

#[test]
fn a_slow_subscriber_drops_events_instead_of_blocking_the_agent() {
    let mut queue = SubscriberQueue::with_capacity(2);

    assert_eq!(queue.push(event_at(1, "gen-1")), PushOutcome::Accepted);
    assert_eq!(queue.push(event_at(2, "gen-1")), PushOutcome::Accepted);
    let third = queue.push(event_at(3, "gen-1"));

    assert_eq!(third, PushOutcome::Dropped { total_dropped: 1 });
    assert_eq!(queue.len(), 2);
    assert!(queue.needs_resync());
    assert_eq!(queue.dropped(), 1);
}

#[test]
fn a_subscriber_that_keeps_up_never_needs_a_resync() {
    let mut queue = SubscriberQueue::with_capacity(2);

    for sequence in 1..=5 {
        assert_eq!(
            queue.push(event_at(sequence, "gen-1")),
            PushOutcome::Accepted
        );
        assert!(queue.pop().is_some());
    }

    assert!(!queue.needs_resync());
    assert!(queue.is_empty());
}

#[test]
fn a_resent_snapshot_clears_the_drop_record() {
    let mut queue = SubscriberQueue::with_capacity(1);
    queue.push(event_at(1, "gen-1"));
    queue.push(event_at(2, "gen-1"));
    assert!(queue.needs_resync());

    queue.clear_resync();

    assert!(!queue.needs_resync());
}

// ---- 時刻の整形 --------------------------------------------------------------

#[test]
fn instants_are_formatted_as_utc_rfc3339() {
    assert_eq!(format_rfc3339_utc(0), "1970-01-01T00:00:00.000Z");
    assert_eq!(T0.to_timestamp().0, "2026-09-20T00:00:00.000Z");
    assert_eq!(
        format_rfc3339_utc(1_789_862_400_000 + 3_661_123),
        "2026-09-20T01:01:01.123Z"
    );
}

#[test]
fn instants_before_the_epoch_are_formatted_without_wrapping() {
    assert_eq!(format_rfc3339_utc(-1), "1969-12-31T23:59:59.999Z");
}

// ---- Mock Backend ------------------------------------------------------------

#[test]
fn every_required_scenario_is_reproducible_without_a_ui() {
    for &scenario in MockScenario::ALL {
        let backend = MockBackend::with_scenario(7, scenario);
        let snapshot = backend.snapshot();

        assert_eq!(snapshot.runners.len(), 2, "{scenario:?}");
        assert!(!snapshot.observed_at.0.is_empty(), "{scenario:?}");
    }
}

#[test]
fn the_same_seed_reproduces_the_same_snapshot() {
    let first = MockBackend::with_scenario(42, MockScenario::Idle).snapshot();
    let second = MockBackend::with_scenario(42, MockScenario::Idle).snapshot();

    assert_eq!(first, second);
}

#[test]
fn a_different_seed_changes_the_identifiers_but_not_the_states() {
    let first = MockBackend::with_scenario(1, MockScenario::Idle).snapshot();
    let second = MockBackend::with_scenario(2, MockScenario::Idle).snapshot();

    assert_ne!(first.node_id, second.node_id);
    assert_eq!(first.effective, second.effective);
}

#[test]
fn the_idle_scenario_is_ready() {
    let snapshot = MockBackend::with_scenario(7, MockScenario::Idle).snapshot();

    assert_eq!(snapshot.effective, EffectiveState::Ready);
}

#[test]
fn the_busy_scenario_is_busy() {
    let snapshot = MockBackend::with_scenario(7, MockScenario::Busy).snapshot();

    assert_eq!(snapshot.effective, EffectiveState::Busy);
}

#[test]
fn the_offline_scenario_is_disconnected() {
    let snapshot = MockBackend::with_scenario(7, MockScenario::Offline).snapshot();

    assert_eq!(snapshot.effective, EffectiveState::Disconnected);
}

#[test]
fn the_unknown_scenario_never_reports_ready() {
    let snapshot = MockBackend::with_scenario(7, MockScenario::Unknown).snapshot();

    assert_eq!(snapshot.effective, EffectiveState::Unknown);
    for runner in &snapshot.runners {
        assert_eq!(runner.remote_availability, RemoteAvailability::Unknown);
    }
}

#[test]
fn the_stale_scenario_reports_unknown_and_keeps_the_last_known_value() {
    let snapshot = MockBackend::with_scenario(7, MockScenario::Stale).snapshot();

    assert_eq!(snapshot.effective, EffectiveState::Unknown);
    for runner in &snapshot.runners {
        assert_eq!(runner.remote_freshness, ObservationFreshness::Stale);
        assert_eq!(
            runner.last_known_availability,
            Some(RemoteAvailability::OnlineIdle)
        );
        // 鮮度が落ちても最後に成功した観測時刻は残す。
        assert!(runner.verified_at.is_some());
    }
}

#[test]
fn the_creating_scenario_holds_an_in_flight_registration() {
    let backend = MockBackend::with_scenario(7, MockScenario::Creating);
    let snapshot = backend.snapshot();

    assert_eq!(snapshot.runners[0].local, LocalRuntime::Starting);
    for runner in &snapshot.runners {
        assert_eq!(runner.remote_presence, RemotePresence::Unchecked);
    }
}

#[test]
fn the_partial_failure_scenario_does_not_report_the_node_as_healthy() {
    let snapshot = MockBackend::with_scenario(7, MockScenario::PartialFailure).snapshot();

    assert_eq!(snapshot.effective, EffectiveState::Partial);
    assert_eq!(snapshot.runners[0].effective, EffectiveState::Ready);
    assert_eq!(
        snapshot.runners[1].remote_error_code,
        Some(ErrorCode::WslGuestUnreachable)
    );
}

#[test]
fn the_rate_limited_scenario_is_unknown_not_offline() {
    let snapshot = MockBackend::with_scenario(7, MockScenario::RateLimited).snapshot();

    assert_eq!(snapshot.effective, EffectiveState::Unknown);
    for runner in &snapshot.runners {
        assert_eq!(runner.remote_error_code, Some(ErrorCode::RateLimited));
        assert_ne!(runner.remote_availability, RemoteAvailability::Offline);
    }
}

#[test]
fn the_reauthentication_scenario_reports_monitoring_unavailable() {
    let snapshot = MockBackend::with_scenario(7, MockScenario::ReauthenticationRequired).snapshot();

    assert_eq!(snapshot.effective, EffectiveState::MonitoringUnavailable);
}

#[test]
fn an_idle_node_becomes_stale_when_the_clock_moves_past_the_threshold() {
    let mut backend = MockBackend::with_scenario(7, MockScenario::Idle);
    assert_eq!(backend.snapshot().effective, EffectiveState::Ready);

    backend.advance_millis(90_001);

    let snapshot = backend.snapshot();
    assert_eq!(snapshot.effective, EffectiveState::Unknown);
    assert_eq!(
        snapshot.runners[0].remote_freshness,
        ObservationFreshness::Stale
    );
}

#[test]
fn a_start_request_records_the_desired_state_and_bumps_the_revision() {
    let mut backend = MockBackend::with_scenario(7, MockScenario::Idle);
    let revision = backend.revision();

    backend
        .request_start(&RequestId::from("req-start"), revision)
        .unwrap();

    assert_eq!(backend.revision(), revision + 1);
    for runner in &backend.snapshot().runners {
        assert_eq!(runner.desired, DesiredState::Running);
    }
}

#[test]
fn a_repeated_start_request_does_not_create_a_second_operation() {
    let mut backend = MockBackend::with_scenario(7, MockScenario::Idle);
    let id = RequestId::from("req-start");
    let revision = backend.revision();

    let first = backend.request_start(&id, revision).unwrap();
    let second = backend.request_start(&id, revision).unwrap();

    assert_eq!(first.operation_id, second.operation_id);
    assert!(second.deduplicated);
    // 再送では revision も event も増えない。
    assert_eq!(backend.revision(), revision + 1);
    assert_eq!(backend.drain_events().len(), 1);
}

#[test]
fn stopping_a_busy_node_stays_in_waiting_for_idle() {
    let mut backend = MockBackend::with_scenario(7, MockScenario::Busy);
    let id = RequestId::from("req-stop");
    let revision = backend.revision();

    backend.request_stop(&id, revision).unwrap();

    assert_eq!(
        backend.operation_phase(&id),
        Some(OperationPhase::WaitingForIdle)
    );
    // 希望状態は Stopped、ローカルはまだ Running を同時に出せる。
    let snapshot = backend.snapshot();
    assert_eq!(snapshot.runners[0].desired, DesiredState::Stopped);
    assert_eq!(snapshot.runners[0].local, LocalRuntime::Running);
}

#[test]
fn stopping_an_idle_node_sends_the_stop_signal() {
    let mut backend = MockBackend::with_scenario(7, MockScenario::Idle);
    let id = RequestId::from("req-stop");
    let revision = backend.revision();

    backend.request_stop(&id, revision).unwrap();

    assert_eq!(
        backend.operation_phase(&id),
        Some(OperationPhase::StopSignal)
    );
}

#[test]
fn stopping_a_node_whose_remote_state_is_unknown_waits() {
    let mut backend = MockBackend::with_scenario(7, MockScenario::RateLimited);
    let id = RequestId::from("req-stop");
    let revision = backend.revision();

    backend.request_stop(&id, revision).unwrap();

    assert_eq!(
        backend.operation_phase(&id),
        Some(OperationPhase::WaitingForIdle)
    );
}

#[test]
fn a_force_stop_needs_a_confirmation_even_in_the_mock() {
    let mut backend = MockBackend::with_scenario(7, MockScenario::Busy);
    let revision = backend.revision();

    let error = backend
        .request_force_stop(&RequestId::from("req-force"), revision, None)
        .unwrap_err();

    assert_eq!(error.code, ErrorCode::RequiresConfirmation);
    // 受理しなかったので revision は動かない。
    assert_eq!(backend.revision(), revision);
}

#[test]
fn mock_events_carry_the_current_generation_and_a_dense_sequence() {
    let mut backend = MockBackend::with_scenario(7, MockScenario::Idle);
    let generation = backend.agent_generation().clone();
    let revision = backend.revision();
    backend
        .request_start(&RequestId::from("req-start"), revision)
        .unwrap();
    backend
        .request_stop(&RequestId::from("req-stop"), revision + 1)
        .unwrap();

    let events = backend.drain_events();
    let mut applier = EventApplier::new(generation.clone(), 1);

    assert_eq!(events.len(), 2);
    for event in &events {
        assert_eq!(event.agent_generation, generation);
        assert_eq!(applier.apply(event), ApplyOutcome::Applied);
    }
}

#[test]
fn a_snapshot_never_carries_a_secret_bearing_field() {
    // 通常の状態 DTO に token を載せないことを、直列化した本文で確かめる。
    let snapshot = MockBackend::with_scenario(7, MockScenario::Idle).snapshot();
    let text = serde_json::to_string(&snapshot).unwrap().to_lowercase();

    for forbidden in ["token", "secret", "password", "authorization", "credential"] {
        assert!(
            !text.contains(forbidden),
            "{forbidden} が snapshot に出ている"
        );
    }
}
