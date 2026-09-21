//! ローカル IPC の待受と接続。
//!
//! UI は Runner、OS シェル、GitHub token、DB を直接操作せず、この crate が
//! 通す固定 method registry だけを経由する（`AGENTS.md` §5.1、
//! `docs/10_IPC_DATA_MODEL.md` §1）。
//!
//! 層の分け方は次のとおりである。
//!
//! - [`name`] と [`security`]: pipe 名と DACL の決め方。OS 呼び出しを含まない
//!   純粋な決定なので、Windows 以外でも試験できる。
//! - [`session`]: 1 接続の読み書き。フレーム上限、handshake 先行、未提供
//!   method の拒否をここで適用する。`AsyncRead + AsyncWrite` に対して書くので、
//!   Linux CI でも `tokio::io::duplex` で試験できる。
//! - [`service`]: method から処理への割り当て。registry は固定で、列挙にない
//!   method はそもそも復号段階で落ちる。
//! - `transport` / `single_instance` / `identity`: Windows 固有の実体。SID の
//!   取得、security attributes 付きの pipe 生成、単一起動の強制を担う。
//!
//! pipe 名は認証ではない（`docs/09_SECURITY.md`「脅威と対策」TH-03）。秘密と
//! して扱わず、接続の可否は DACL と remote client 拒否で決める。

#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used, clippy::panic))]

pub mod name;
pub mod security;
pub mod service;
pub mod session;

#[cfg(windows)]
pub mod identity;
#[cfg(windows)]
pub mod single_instance;
#[cfg(windows)]
pub mod transport;

#[cfg(windows)]
pub mod client;
