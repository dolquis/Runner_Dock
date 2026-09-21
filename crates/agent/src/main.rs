//! Windows User Agent の実行ファイル。
//!
//! 非昇格で動き、同一 SID あたり 1 インスタンスに限る（ADR-003、
//! `docs/03_ARCHITECTURE.md` §3）。待受は所有者だけが繋げる named pipe で、
//! 受け付けるのは固定 registry の method だけである。
//!
//! この段階では GitHub へ接続せず、Runner を登録・起動せず、WSL の状態を
//! 変更しない。IPC の向こう側は LF-002 の Mock ドメインで、実 Backend は
//! LF-007 以降で差し替える。

use std::io;

use runnerdock_protocol::PROTOCOL_MAJOR;
use runnerdock_protocol::dto::HandshakeRequest;

fn main() -> io::Result<()> {
    tracing_subscriber::fmt()
        // 機械可読な自己申告を stdout へ出すため、診断ログは stderr へ分ける。
        // guest 側と同じ分離にしておく。
        .with_writer(io::stderr)
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_env("RUNNERDOCK_LOG")
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let peer = HandshakeRequest {
        protocol_major: PROTOCOL_MAJOR,
        implementation_version: env!("CARGO_PKG_VERSION").to_owned(),
    };
    tracing::info!(
        protocol_major = peer.protocol_major,
        implementation_version = %peer.implementation_version,
        "runnerdock agent self-check"
    );

    // 起動確認を機械可読な形でも出す。ここに秘密は含めない。
    let line = serde_json::to_string(&peer).map_err(io::Error::other)?;
    println!("{line}");

    // `--self-check` は起動できることだけを確かめる経路で、待受へ入らない。
    // 非 Windows では pipe を作れないので、同じ経路で終える。
    if std::env::args().any(|arg| arg == "--self-check") {
        return Ok(());
    }
    serve()
}

#[cfg(windows)]
fn serve() -> io::Result<()> {
    use runnerdock_core::mock::{MockBackend, MockScenario};
    use runnerdock_ipc::identity::{current_user_sid, is_elevated};
    use runnerdock_ipc::service::MockAgentService;
    use runnerdock_ipc::session::serve_connection;
    use runnerdock_ipc::single_instance::{LockError, SingleInstanceLock};
    use runnerdock_ipc::transport::PipeListener;

    // 日常運用の Agent を昇格して動かさない（ADR-003）。昇格したまま待受けると、
    // 同じ SID の非昇格 GUI から昇格権限の操作へ繋がってしまう。
    if is_elevated()? {
        tracing::error!("昇格した状態では待受を開始しない。通常の権限で起動する");
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "Agent は非昇格で動かす",
        ));
    }

    let sid = current_user_sid()?;
    // 単一起動は mutex と pipe の両方で確かめる。片方だけでは、取り残された
    // ロックや、別セッションからの二重起動を取りこぼす。
    let lock = match SingleInstanceLock::acquire(&sid) {
        Ok(lock) => lock,
        Err(LockError::AlreadyRunning) => {
            tracing::info!("同名の単一起動ロックが既にあるため起動しない");
            return Ok(());
        }
        Err(LockError::MalformedSid) => {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "SID からロック名を決められない",
            ));
        }
        Err(LockError::Os(error)) => return Err(error),
    };

    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    runtime.block_on(async move {
        let mut listener = match PipeListener::bind(&sid) {
            Ok(listener) => listener,
            // 別のログオンセッションの Agent が同じ名前で待受けている場合。
            // mutex 側と同じく、二重起動の防止が働いた結果として終える。
            Err(error) if is_already_listening(&error) => {
                tracing::info!(kind = ?error.kind(), "同名の待受が既にあるため起動しない");
                return Ok(());
            }
            Err(error) => return Err(error),
        };
        tracing::info!(pipe = listener.name(), lock = lock.name(), "待受を開始した");

        let mut service = MockAgentService::new(MockBackend::with_scenario(1, MockScenario::Idle));
        loop {
            tokio::select! {
                // Ctrl+C は Agent 自身の終了要求。GUI の終了とは別物である。
                signal = tokio::signal::ctrl_c() => {
                    signal?;
                    tracing::info!("停止要求を受け取った");
                    return Ok(());
                }
                accepted = listener.accept() => {
                    match accepted {
                        Ok(stream) => {
                            // 1 接続ずつ処理する。同時接続の多重化は LF-009 で扱う。
                            let end = serve_connection(stream, &mut service).await;
                            tracing::info!(?end, "接続を終えた");
                        }
                        Err(error) => {
                            // 1 接続の失敗で Agent を落とさない。次の接続を待つ。
                            tracing::warn!(kind = ?error.kind(), "接続を受け付けられなかった");
                        }
                    }
                }
            }
        }
    })
}

/// 待受を作れない理由が「既に同名の待受がある」か。
///
/// `ERROR_ACCESS_DENIED` / `ERROR_ALREADY_EXISTS` / `ERROR_PIPE_BUSY` だけを
/// そう扱う。他の失敗を二重起動の防止へ読み替えると、待受を持てていない状態を
/// 正常終了として隠してしまう。
#[cfg(windows)]
fn is_already_listening(error: &io::Error) -> bool {
    matches!(error.raw_os_error(), Some(5 | 183 | 231))
}

#[cfg(not(windows))]
fn serve() -> io::Result<()> {
    // Agent は Windows のユーザーセッションで動く前提である（ADR-003）。
    // 他の OS では待受を持たないことを明示して終える。
    tracing::info!("この OS では待受を提供しない");
    Ok(())
}
