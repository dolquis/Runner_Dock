//! Agent への橋渡し。
//!
//! GUI はまず既存の Agent へ繋ぎ、繋がらないときだけ製品同梱の Agent を起こす
//! （`docs/03_ARCHITECTURE.md` §3）。起こした Agent は GUI のプロセスツリー、
//! Job Object、コンソールから切り離す。GUI を閉じる操作は Agent の終了では
//! ないためである（`docs/07_WINDOWS_WSL_BACKENDS.md`）。
//!
//! UI から受け取るのは「繋げ」「状態を寄こせ」という意図だけで、実行ファイルの
//! path も引数も pipe 名も UI からは渡させない。

use std::io;

use runnerdock_protocol::dto::{HandshakeResult, NodeSnapshot};
use runnerdock_protocol::error::{ErrorCode, ErrorPayload};
use serde_json::json;

/// UI へ返せる形へ整えた失敗。秘密も OS の詳細も載せない。
fn refuse(code: ErrorCode, reason: &str) -> ErrorPayload {
    ErrorPayload::new(code).with_detail("reason", json!(reason))
}

/// Agent の待受へ繋ぎ、handshake と snapshot を取る。
///
/// 待受が無ければ Agent を起こしてから繋ぎ直す。
///
/// # Errors
///
/// Agent を起こせないとき、起こしても繋がらないとき、Agent が要求を拒否した
/// ときに、機械判定できる [`ErrorPayload`] を返す。
#[cfg(windows)]
pub async fn connect_or_start() -> Result<(HandshakeResult, NodeSnapshot), ErrorPayload> {
    use runnerdock_ipc::client::{AgentClient, ClientError, connect_with_retry};
    use runnerdock_ipc::identity::current_user_sid;

    let sid =
        current_user_sid().map_err(|_| refuse(ErrorCode::PermissionOrPolicy, "sidUnavailable"))?;

    let connected = match AgentClient::connect(&sid).await {
        Ok(connected) => connected,
        Err(ClientError::Connect(_)) => {
            // 待受が無い。製品同梱の Agent を起こしてから待つ。
            start_detached_agent().map_err(|error| {
                refuse(
                    ErrorCode::PermissionOrPolicy,
                    match error.kind() {
                        io::ErrorKind::PermissionDenied => "agentBreakawayDenied",
                        io::ErrorKind::NotFound => "agentBinaryMissing",
                        _ => "agentStartFailed",
                    },
                )
            })?;
            connect_with_retry(&sid, START_CONNECT_ATTEMPTS, START_CONNECT_INTERVAL)
                .await
                .map_err(|_| refuse(ErrorCode::PermissionOrPolicy, "agentNotListening"))?
        }
        Err(error) => return Err(to_payload(error)),
    };

    let (mut client, handshake) = connected;
    let snapshot = client.node_snapshot().await.map_err(to_payload)?;
    Ok((handshake, snapshot))
}

/// Windows 以外では Agent を持たない。
///
/// # Errors
///
/// 常に [`ErrorCode::PermissionOrPolicy`] を返す。
#[cfg(not(windows))]
pub async fn connect_or_start() -> Result<(HandshakeResult, NodeSnapshot), ErrorPayload> {
    Err(refuse(ErrorCode::PermissionOrPolicy, "unsupportedPlatform"))
}

/// 起動直後の Agent を待つ回数と間隔。
#[cfg(windows)]
const START_CONNECT_ATTEMPTS: u32 = 20;
#[cfg(windows)]
const START_CONNECT_INTERVAL: std::time::Duration = std::time::Duration::from_millis(100);

#[cfg(windows)]
fn to_payload(error: runnerdock_ipc::client::ClientError) -> ErrorPayload {
    use runnerdock_ipc::client::ClientError;

    match error {
        // Agent が返した型付きエラーは、丸めずそのまま UI へ渡す。
        ClientError::Refused(payload) => *payload,
        ClientError::Connect(_) => refuse(ErrorCode::PermissionOrPolicy, "agentNotListening"),
        ClientError::Io(_) => refuse(ErrorCode::StatusStale, "agentDisconnected"),
        ClientError::MalformedResponse => {
            refuse(ErrorCode::ProtocolMismatch, "malformedAgentResponse")
        }
    }
}

/// 製品同梱の Agent を、GUI から切り離して起動する。
///
/// - `DETACHED_PROCESS`: GUI のコンソールを引き継がせない。
/// - `CREATE_NEW_PROCESS_GROUP`: GUI への Ctrl+C が Agent へ伝わらないようにする。
/// - `CREATE_BREAKAWAY_FROM_JOB`: GUI が属する Job Object の終了に巻き込まれない
///   ようにする。GUI を強制終了しても Agent を道連れにしないための指定である。
///
/// 標準入出力はすべて閉じる。`DETACHED_PROCESS` だけでは handle の継承は止まらない。
///
/// 実行ファイルは GUI と同じディレクトリから決め打ちで探す。path を引数や設定から
/// 受け取らないのは、UI 側から任意の実行ファイルを起動する経路を作らないためである。
///
/// # Errors
///
/// 実行ファイルが見つからないとき、Job Object の離脱を許可されていないとき
/// （`ERROR_ACCESS_DENIED`）に返す。離脱できないまま起動へ倒さない。
#[cfg(windows)]
fn start_detached_agent() -> io::Result<()> {
    use std::os::windows::process::CommandExt;
    use std::process::{Command, Stdio};

    use windows_sys::Win32::System::Threading::{
        CREATE_BREAKAWAY_FROM_JOB, CREATE_NEW_PROCESS_GROUP, DETACHED_PROCESS,
    };

    let executable = std::env::current_exe()?
        .parent()
        .map(|directory| directory.join(AGENT_EXECUTABLE))
        .ok_or_else(|| io::Error::from(io::ErrorKind::NotFound))?;
    if !executable.is_file() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            "Agent の実行ファイルが見つからない",
        ));
    }

    Command::new(executable)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .creation_flags(DETACHED_PROCESS | CREATE_NEW_PROCESS_GROUP | CREATE_BREAKAWAY_FROM_JOB)
        .spawn()
        .map(|_| ())
}

/// 同梱する Agent の実行ファイル名。
#[cfg(windows)]
const AGENT_EXECUTABLE: &str = "runnerdock-agent.exe";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn refusals_carry_a_machine_readable_reason() {
        let payload = refuse(ErrorCode::PermissionOrPolicy, "agentBreakawayDenied");

        assert_eq!(payload.code, ErrorCode::PermissionOrPolicy);
        assert_eq!(
            payload.details.get("reason"),
            Some(&json!("agentBreakawayDenied"))
        );
        assert!(!payload.retryable);
    }

    #[cfg(windows)]
    #[test]
    fn the_agent_executable_is_resolved_next_to_the_shell() {
        // path を外から受け取らないことを、名前が定数であることで示す。
        assert_eq!(AGENT_EXECUTABLE, "runnerdock-agent.exe");
        assert!(!AGENT_EXECUTABLE.contains(std::path::MAIN_SEPARATOR));
    }

    #[cfg(windows)]
    #[test]
    fn missing_agent_binary_is_reported_instead_of_being_ignored() {
        // 開発ビルドでは shell の隣に Agent が無いことがある。その場合に
        // 成功へ倒さないことを確かめる。置かれている場合は起動を試さない。
        let executable = std::env::current_exe()
            .unwrap()
            .parent()
            .map(|directory| directory.join(AGENT_EXECUTABLE));
        if executable.is_some_and(|path| path.is_file()) {
            return;
        }

        let error = start_detached_agent().unwrap_err();

        assert_eq!(error.kind(), std::io::ErrorKind::NotFound);
    }
}
