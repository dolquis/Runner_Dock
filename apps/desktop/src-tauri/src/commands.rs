//! UI へ公開する型付き command。
//!
//! 汎用 `exec` / `readFile` / `writeFile` / `sql` は公開しない
//! （`docs/10_IPC_DATA_MODEL.md` §4）。追加する command は、引数と戻り値を
//! 具体的な型で表し、UI へ OS 権限を渡さない形にする。

use runnerdock_protocol::PROTOCOL_MAJOR;
use runnerdock_protocol::dto::HandshakeRequest;

/// UI へ返す shell の自己申告。秘密も OS の詳細も含めない。
#[tauri::command]
pub fn shell_peer() -> HandshakeRequest {
    HandshakeRequest {
        protocol_major: PROTOCOL_MAJOR,
        implementation_version: env!("CARGO_PKG_VERSION").to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reports_the_protocol_major_the_shell_was_built_against() {
        assert_eq!(shell_peer().protocol_major, PROTOCOL_MAJOR);
    }
}
