//! Desktop の Rust bridge。
//!
//! UI へ公開するのは型付き command だけで、OS シェル、Runner、token、DB への
//! 直接の経路は作らない（AGENTS.md §5.1）。Agent との往復は [`agent`] が持つ。

// テストでは失敗時に落ちてよい。製品コード側の unwrap / expect だけを止める。
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]

pub mod agent;
pub mod commands;

/// Tauri アプリケーションを起動する。
///
/// # Panics
///
/// Tauri の context 生成または起動に失敗した場合に panic する。起動できない
/// 状態で UI を出しても操作できないため、ここでは復旧を試みない。
#[cfg_attr(mobile, tauri::mobile_entry_point)]
#[expect(
    clippy::expect_used,
    reason = "起動に失敗した時点で UI を出す意味がないため、明示的に落とす"
)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            commands::shell_peer,
            commands::agent_status
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
