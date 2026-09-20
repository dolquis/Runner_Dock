//! Runner Dock の純粋ドメイン。
//!
//! Tauri、WSL コマンド、GitHub HTTP クライアントに依存しない（AGENTS.md §5.2）。
//! 時計は [`clock::Clock`] で注入し、観測値は呼び出し側が取得した JSON 断片として
//! 受け取る。この crate は秘密情報を保持も出力もしない。

#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used, clippy::panic))]

pub mod clock;
pub mod effective;
pub mod events;
pub mod freshness;
pub mod mock;
pub mod operation;
pub mod remote;

#[cfg(test)]
mod tests;
