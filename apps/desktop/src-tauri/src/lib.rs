//! Desktop の Rust bridge。
//!
//! LF-001 では Agent へ接続しない。UI が「どの契約版の shell に載っているか」
//! を確認できる読取 command だけを公開する。

// テストでは失敗時に落ちてよい。製品コード側の unwrap / expect だけを止める。
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]

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
        .invoke_handler(tauri::generate_handler![commands::shell_peer])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
