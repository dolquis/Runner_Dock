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

/// 受理済み Operation の集合。Agent が 1 つだけ持つ。
#[derive(Debug, Default)]
pub struct OperationStore {
    by_request: HashMap<RequestId, OperationId>,
    records: HashMap<OperationId, OperationRecord>,
    next_id: u64,
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
            && request
                .confirmation
                .as_ref()
                .is_none_or(|value| value.trim().is_empty())
        {
            // API が不明な場合も確認なしの停止へ進めない。
            //
            // ここは「空でない応答があるか」までしか見ない。challenge を Agent 側で
            // 発行して照合する経路は IPC を実装する LF-005 の範囲で、この段階では
            // 確認を省略できないことだけを保証する。
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
