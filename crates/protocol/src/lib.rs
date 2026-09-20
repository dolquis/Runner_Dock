//! Runner Dock の IPC ワイヤー契約。
//!
//! LF-001 の範囲は `system.handshake` の版判定だけである。メソッド一覧、
//! イベント、エラー契約、TypeScript 生成は LF-002 で追加する
//! （`docs/10_IPC_DATA_MODEL.md` §4）。

// テストでは失敗時に落ちてよい。製品コード側の unwrap / expect だけを止める。
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]

pub mod handshake;

pub use handshake::{Handshake, HandshakeRejection, PROTOCOL_VERSION, Peer, is_compatible, review};
