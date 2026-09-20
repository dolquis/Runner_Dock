//! Runner Dock の純粋ドメイン。
//!
//! この crate は OS 操作、Tauri、HTTP クライアント、SQLite を参照しない。
//! 実行・時計・秘密情報・保存先は呼び出し側がインターフェースで注入する
//! （`docs/03_ARCHITECTURE.md` §1）。
//!
//! LF-001 の範囲は Backend の識別子だけである。Node / Runner の希望状態と
//! 観測状態、Operation、Reconciler は LF-002 で追加する。

// テストでは失敗時に落ちてよい。製品コード側の unwrap / expect だけを止める。
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]

pub mod backend;

pub use backend::{BackendKind, ParseBackendKindError};
