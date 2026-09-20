//! イベントの適用と配信。`docs/03_ARCHITECTURE.md` §5、`docs/10_IPC_DATA_MODEL.md` §9。
//!
//! Agent 世代が変わったら旧世代の event を捨てる。欠落したら差分を当てずに snapshot
//! へ戻す。遅い購読者が Agent 全体を止めないよう、キューは上限付きで落とす。

use std::collections::VecDeque;

use runnerdock_protocol::ids::{AgentGeneration, DecimalU64};
use runnerdock_protocol::message::{Event, EventKind};

/// 1 件の event を適用した結果。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ApplyOutcome {
    /// 期待どおりの連番。状態へ適用する。
    Applied,
    /// 別世代の Agent が出した event。捨てる。
    DroppedForeignGeneration { found: AgentGeneration },
    /// 既に適用済みより古い連番。新しい状態へ古い差分を当てない。
    DroppedStale { sequence: u64, expected: u64 },
    /// 連番が飛んだ。差分を当てずに snapshot を取り直す。
    ResyncRequired { gap_from: u64, received: u64 },
}

/// 受信側の event 適用器。GUI と CLI がそれぞれ 1 つ持つ。
#[derive(Debug, Clone)]
pub struct EventApplier {
    generation: AgentGeneration,
    next_sequence: u64,
}

impl EventApplier {
    /// snapshot を受け取った直後の状態から始める。
    ///
    /// `next_sequence` は snapshot に対応する連番の次の値。
    #[must_use]
    pub const fn new(generation: AgentGeneration, next_sequence: u64) -> Self {
        Self {
            generation,
            next_sequence,
        }
    }

    #[must_use]
    pub fn generation(&self) -> &AgentGeneration {
        &self.generation
    }

    #[must_use]
    pub const fn next_sequence(&self) -> u64 {
        self.next_sequence
    }

    /// Agent が再起動して世代が変わった。snapshot を取り直したうえで呼ぶ。
    pub fn rebind(&mut self, generation: AgentGeneration, next_sequence: u64) {
        self.generation = generation;
        self.next_sequence = next_sequence;
    }

    /// event を 1 件適用してよいか判定する。
    pub fn apply(&mut self, event: &Event) -> ApplyOutcome {
        if event.agent_generation != self.generation {
            return ApplyOutcome::DroppedForeignGeneration {
                found: event.agent_generation.clone(),
            };
        }

        let sequence = event.sequence.get();
        if sequence < self.next_sequence {
            // 時計が戻っても連番は戻らない。順序は sequence だけで判断する。
            return ApplyOutcome::DroppedStale {
                sequence,
                expected: self.next_sequence,
            };
        }
        if sequence > self.next_sequence {
            return ApplyOutcome::ResyncRequired {
                gap_from: self.next_sequence,
                received: sequence,
            };
        }

        self.next_sequence += 1;
        ApplyOutcome::Applied
    }
}

/// 購読者キューへ入れた結果。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PushOutcome {
    Accepted,
    /// 上限に達したので落とした。累計の欠落件数を返す。
    Dropped {
        total_dropped: u64,
    },
}

/// 購読者ごとの上限付きキュー。
///
/// 満杯でも送信側を待たせない。落としたものの扱いは `docs/10_IPC_DATA_MODEL.md` §9 に
/// 従って 2 つに分ける。重要状態イベントを落としたら `ResyncRequired` を送る必要が
/// あり（[`needs_resync`](Self::needs_resync)）、ログを間引いた場合は件数を表示する
/// （[`dropped_logs`](Self::dropped_logs)）。ログ 1 件の欠落で snapshot を取り直さない。
#[derive(Debug, Clone)]
pub struct SubscriberQueue {
    capacity: usize,
    items: VecDeque<Event>,
    dropped_state: u64,
    dropped_logs: u64,
}

impl SubscriberQueue {
    /// 容量を指定して作る。容量 0 は 1 として扱う。
    #[must_use]
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            capacity: capacity.max(1),
            items: VecDeque::new(),
            dropped_state: 0,
            dropped_logs: 0,
        }
    }

    /// event を 1 件入れる。満杯なら新しい方を落とす。
    pub fn push(&mut self, event: Event) -> PushOutcome {
        if self.items.len() >= self.capacity {
            // ログの間引きと状態の取りこぼしを別々に数える。
            if event.event == EventKind::LogAppended {
                self.dropped_logs += 1;
            } else {
                self.dropped_state += 1;
            }
            return PushOutcome::Dropped {
                total_dropped: self.dropped_state + self.dropped_logs,
            };
        }
        self.items.push_back(event);
        PushOutcome::Accepted
    }

    /// 次の event を取り出す。
    pub fn pop(&mut self) -> Option<Event> {
        self.items.pop_front()
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.items.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// 落とした状態イベントの累計。
    #[must_use]
    pub const fn dropped_state(&self) -> u64 {
        self.dropped_state
    }

    /// 間引いたログの累計。UI へ件数として表示する。
    #[must_use]
    pub const fn dropped_logs(&self) -> u64 {
        self.dropped_logs
    }

    /// 落とした累計（状態とログの合計）。
    #[must_use]
    pub const fn dropped(&self) -> u64 {
        self.dropped_state + self.dropped_logs
    }

    /// 状態イベントを落としたか。落としていれば snapshot へ戻す必要がある。
    ///
    /// ログだけを間引いた場合は真にしない。件数表示で足りるため。
    #[must_use]
    pub const fn needs_resync(&self) -> bool {
        self.dropped_state > 0
    }

    /// snapshot を送り直したので状態イベントの欠落記録を消す。
    ///
    /// ログの間引き件数は snapshot では埋まらないので残す。
    pub const fn clear_resync(&mut self) {
        self.dropped_state = 0;
    }
}

/// 送信側の連番採番。Agent が 1 つだけ持つ。
#[derive(Debug, Clone)]
pub struct SequenceSource {
    generation: AgentGeneration,
    next: u64,
}

impl SequenceSource {
    #[must_use]
    pub const fn new(generation: AgentGeneration, start: u64) -> Self {
        Self {
            generation,
            next: start,
        }
    }

    /// 次の連番を払い出す。
    pub const fn take(&mut self) -> DecimalU64 {
        let value = self.next;
        self.next += 1;
        DecimalU64::new(value)
    }

    #[must_use]
    pub fn generation(&self) -> &AgentGeneration {
        &self.generation
    }
}
