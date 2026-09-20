//! Operation の受理と段階。`docs/05_DOMAIN_STATE.md` §5 / §6。
//!
//! 同じ `requestId` の再受信で重複副作用を起こさない。外部 API 呼出しと保存を 1 つの
//! 原子操作として扱わないため、登録応答で得た remote ID は Operation 側に記録し、
//! 保存に失敗した再試行では再登録せず照合へ回す（`docs/10_IPC_DATA_MODEL.md` §7）。

use std::collections::HashMap;

use runnerdock_protocol::dto::{
    OperationAccepted, OperationFailure, OperationKind, OperationPhase, OperationSnapshot,
    RemoteAvailability,
};
use runnerdock_protocol::error::{ErrorCode, ErrorPayload};
use runnerdock_protocol::ids::{DecimalU64, OperationId, RequestId};

use crate::clock::UnixMillis;

/// Operation の受理要求。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OperationRequest {
    pub request_id: RequestId,
    pub kind: OperationKind,
    pub target_id: String,
    /// 直前に読んだ snapshot の revision。
    pub expected_revision: u64,
    /// 強制停止の確認応答。`node.forceStop` 以外では使わない。
    pub confirmation: Option<String>,
}

/// 停止判断に使う、対象の最新観測。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StopReadiness {
    pub availability: RemoteAvailability,
    /// 観測が fresh でなければ Idle と断定しない。
    pub is_fresh: bool,
}

impl StopReadiness {
    /// 即時に停止信号を送ってよいか。
    ///
    /// `OnlineIdle` かつ fresh のときだけ真。Busy と Unknown、鮮度切れは保留する。
    #[must_use]
    pub const fn is_confirmed_idle(self) -> bool {
        self.is_fresh && matches!(self.availability, RemoteAvailability::OnlineIdle)
    }
}

/// core 側が保持する Operation の記録。ワイヤー DTO より項目が多い。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OperationRecord {
    pub operation_id: OperationId,
    pub request_id: RequestId,
    pub kind: OperationKind,
    pub target_id: String,
    pub phase: OperationPhase,
    pub started_at: UnixMillis,
    pub finished_at: Option<UnixMillis>,
    /// 登録応答で得た GitHub 側 Runner ID。保存に失敗しても、ここに残った値で照合する。
    pub registered_remote_id: Option<DecimalU64>,
    pub failures: Vec<OperationFailure>,
}

impl OperationRecord {
    /// ワイヤー DTO へ変換する。
    #[must_use]
    pub fn to_snapshot(&self) -> OperationSnapshot {
        OperationSnapshot {
            operation_id: self.operation_id.clone(),
            kind: self.kind,
            phase: self.phase,
            target_id: self.target_id.clone(),
            started_at: self.started_at.to_timestamp(),
            finished_at: self.finished_at.map(UnixMillis::to_timestamp),
            failures: self.failures.clone(),
        }
    }
}

/// 強制停止の確認 challenge。
///
/// `docs/10_IPC_DATA_MODEL.md` §4 は `node.forceStop` に confirmation challenge を
/// 必須とし、`docs/05_DOMAIN_STATE.md` §6 は確認ダイアログに対象と観測時刻を出すと
/// している。「何か入力したか」ではなく、Agent が発行したこの値との完全一致で判定する。
///
/// `token` の生成は呼び出し側の責務である。core は乱数源を持たないため、Agent が
/// OS の乱数で作った値を渡す。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ForceStopChallenge {
    pub token: String,
    /// この challenge が有効な対象。別の Node の確認を使い回させない。
    pub target_id: String,
    /// 発行時点の revision。状態が動いたら無効になる。
    pub expected_revision: u64,
    pub issued_at: UnixMillis,
    pub expires_at: UnixMillis,
}

impl ForceStopChallenge {
    /// 発行から失効までの時間。確認ダイアログを開いたまま放置された値を通さない。
    pub const TTL_MILLIS: i64 = 120_000;

    /// この時刻で使えるか。
    #[must_use]
    pub const fn is_valid_at(&self, now: UnixMillis) -> bool {
        now.elapsed_since(self.expires_at) <= 0
    }
}

/// 受理済み Operation の集合。Agent が 1 つだけ持つ。
#[derive(Debug, Default)]
pub struct OperationStore {
    by_request: HashMap<RequestId, OperationId>,
    records: HashMap<OperationId, OperationRecord>,
    next_id: u64,
    /// 発行済みで未使用の強制停止 challenge。対象ごとに 1 件だけ持つ。
    pending_challenges: HashMap<String, ForceStopChallenge>,
}

impl OperationStore {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// 要求を受理する。
    ///
    /// 同じ `requestId` を再受信した場合は、新しい副作用を作らずに既存 Operation を
    /// 返す（`deduplicated = true`）。
    ///
    /// # Errors
    ///
    /// revision がずれていれば `REVISION_CONFLICT`、強制停止で確認応答が無ければ
    /// `REQUIRES_CONFIRMATION` を返す。
    pub fn submit(
        &mut self,
        request: &OperationRequest,
        current_revision: u64,
        readiness: StopReadiness,
        now: UnixMillis,
    ) -> Result<OperationAccepted, ErrorPayload> {
        // 冪等キーの照合を最初に行う。受理後に revision が進むため、これを後ろへ
        // 置くと正当な再送が `REVISION_CONFLICT` で落ちる。
        //
        // ただし「同じ鍵」だけでは再送と認めない。kind と対象が違う要求を再送として
        // 通すと、確認や revision のゲートを鍵の使い回しで迂回できてしまう。
        if let Some(existing_id) = self.by_request.get(&request.request_id) {
            let existing = self.records.get(existing_id);
            let is_same_request = existing.is_some_and(|record| {
                record.kind == request.kind && record.target_id == request.target_id
            });
            if !is_same_request {
                return Err(ErrorPayload::new(ErrorCode::RequestIdConflict));
            }
            return Ok(OperationAccepted {
                operation_id: existing_id.clone(),
                accepted: true,
                deduplicated: true,
            });
        }

        if request.expected_revision != current_revision {
            return Err(ErrorPayload::new(ErrorCode::RevisionConflict)
                .with_detail(
                    "expectedRevision",
                    request.expected_revision.to_string().into(),
                )
                .with_detail("currentRevision", current_revision.to_string().into()));
        }

        if request.kind == OperationKind::NodeForceStop
            && !self.consume_challenge(request, current_revision, now)
        {
            // API が不明な場合も確認なしの停止へ進めない。発行済み challenge との
            // 完全一致だけを確認済みとみなす。
            return Err(ErrorPayload::new(ErrorCode::RequiresConfirmation));
        }

        let operation_id = self.allocate_id();
        let record = OperationRecord {
            operation_id: operation_id.clone(),
            request_id: request.request_id.clone(),
            kind: request.kind,
            target_id: request.target_id.clone(),
            phase: initial_phase(request.kind, readiness),
            started_at: now,
            finished_at: None,
            registered_remote_id: None,
            failures: Vec::new(),
        };

        self.by_request
            .insert(request.request_id.clone(), operation_id.clone());
        self.records.insert(operation_id.clone(), record);

        Ok(OperationAccepted {
            operation_id,
            accepted: true,
            deduplicated: false,
        })
    }

    /// 強制停止の確認 challenge を発行する。
    ///
    /// `token` は呼び出し側が OS の乱数で作る。対象ごとに 1 件だけ保持し、発行し直すと
    /// 前の値は無効になる。
    pub fn issue_force_stop_challenge(
        &mut self,
        token: String,
        target_id: &str,
        expected_revision: u64,
        now: UnixMillis,
    ) -> ForceStopChallenge {
        let challenge = ForceStopChallenge {
            token,
            target_id: target_id.to_owned(),
            expected_revision,
            issued_at: now,
            expires_at: now.plus_millis(ForceStopChallenge::TTL_MILLIS),
        };
        self.pending_challenges
            .insert(target_id.to_owned(), challenge.clone());
        challenge
    }

    /// 要求に付いた確認応答が、発行済み challenge と一致するか判定する。
    ///
    /// 一致したら使い切る。同じ値の再送で 2 回目の強制停止を通さない。
    fn consume_challenge(
        &mut self,
        request: &OperationRequest,
        current_revision: u64,
        now: UnixMillis,
    ) -> bool {
        let Some(answer) = request.confirmation.as_ref() else {
            return false;
        };
        let Some(challenge) = self.pending_challenges.get(&request.target_id) else {
            return false;
        };

        let matches = challenge.token == *answer
            && challenge.target_id == request.target_id
            && challenge.expected_revision == current_revision
            && challenge.is_valid_at(now);
        if !matches {
            return false;
        }

        self.pending_challenges.remove(&request.target_id);
        true
    }

    /// 発行済みで未使用の challenge を読む。
    #[must_use]
    pub fn pending_challenge(&self, target_id: &str) -> Option<&ForceStopChallenge> {
        self.pending_challenges.get(target_id)
    }

    /// 登録応答で得た remote ID を記録する。ローカル保存より前に呼ぶ。
    ///
    /// 段階は動かさない。`Registering` の次に `Starting` を経てから `Verifying` へ
    /// 進むのは呼び出し側の責務で（`docs/05_DOMAIN_STATE.md` §5）、ここで飛ばすと
    /// 遷移図に無い経路ができる。
    pub fn record_registration(&mut self, operation_id: &OperationId, remote_id: DecimalU64) {
        if let Some(record) = self.records.get_mut(operation_id) {
            record.registered_remote_id = Some(remote_id);
        }
    }

    /// 段階を進める。
    ///
    /// 終端に達した Operation は動かさない。結果を記録したあとに巻き戻すと、
    /// 完了した操作が再び進行中に見える。
    pub fn set_phase(
        &mut self,
        operation_id: &OperationId,
        phase: OperationPhase,
        now: UnixMillis,
    ) -> bool {
        let Some(record) = self.records.get_mut(operation_id) else {
            return false;
        };
        if record.phase.is_terminal() {
            return false;
        }
        record.phase = phase;
        if phase.is_terminal() {
            record.finished_at = Some(now);
        }
        true
    }

    /// 片側の失敗を記録する。成功側を巻き戻さない。
    pub fn push_failure(&mut self, operation_id: &OperationId, failure: OperationFailure) {
        if let Some(record) = self.records.get_mut(operation_id) {
            record.failures.push(failure);
        }
    }

    #[must_use]
    pub fn get(&self, operation_id: &OperationId) -> Option<&OperationRecord> {
        self.records.get(operation_id)
    }

    /// 冪等キーから既存 Operation を引く。中断後の再開でここから復旧する。
    #[must_use]
    pub fn find_by_request(&self, request_id: &RequestId) -> Option<&OperationRecord> {
        self.by_request
            .get(request_id)
            .and_then(|id| self.records.get(id))
    }

    fn allocate_id(&mut self) -> OperationId {
        self.next_id += 1;
        OperationId(format!("op-{:04}", self.next_id))
    }
}

/// 受理直後の段階を決める。
fn initial_phase(kind: OperationKind, readiness: StopReadiness) -> OperationPhase {
    match kind {
        OperationKind::NodeStart | OperationKind::RunnerCreate | OperationKind::RunnerRemove => {
            OperationPhase::Requested
        }
        OperationKind::NodeStop => {
            if readiness.is_confirmed_idle() {
                OperationPhase::StopSignal
            } else {
                // Busy も Unknown も鮮度切れも保留する。Unknown を Idle にしない。
                OperationPhase::WaitingForIdle
            }
        }
        // 確認済みの強制停止。idle 待ちをしない代わりに中断の可能性を UI が示す。
        OperationKind::NodeForceStop => OperationPhase::StopSignal,
    }
}
