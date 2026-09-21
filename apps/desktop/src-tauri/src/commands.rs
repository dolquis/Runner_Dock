//! UI へ公開する型付き command。
//!
//! 汎用 `exec` / `readFile` / `writeFile` / `sql` は公開しない
//! （`docs/10_IPC_DATA_MODEL.md` §4）。追加する command は、引数と戻り値を
//! 具体的な型で表し、UI へ OS 権限を渡さない形にする。

use runnerdock_protocol::PROTOCOL_MAJOR;
use runnerdock_protocol::dto::{AgentStatus, HandshakeRequest};
use runnerdock_protocol::error::ErrorPayload;

use crate::agent;

/// UI へ返す shell の自己申告。秘密も OS の詳細も含めない。
#[tauri::command]
pub fn shell_peer() -> HandshakeRequest {
    HandshakeRequest {
        protocol_major: PROTOCOL_MAJOR,
        implementation_version: env!("CARGO_PKG_VERSION").to_owned(),
    }
}

/// Agent へ繋ぎ、handshake と現在の snapshot を返す。
///
/// UI から渡せる引数は無い。pipe 名も実行ファイルの path も UI が決めない。
/// 待受が無ければ Agent を起こすが、その判断もここで閉じる
/// （`docs/03_ARCHITECTURE.md` §3、AGENTS.md §5.1）。
///
/// # Errors
///
/// Agent へ繋げないとき、Agent が要求を拒否したときに [`ErrorPayload`] を返す。
/// 取得できなかったことを既定値で埋めない。
#[tauri::command]
pub async fn agent_status() -> Result<AgentStatus, ErrorPayload> {
    let (handshake, snapshot) = agent::connect_or_start().await?;
    Ok(AgentStatus {
        handshake,
        snapshot,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reports_the_protocol_major_the_shell_was_built_against() {
        assert_eq!(shell_peer().protocol_major, PROTOCOL_MAJOR);
    }
}
