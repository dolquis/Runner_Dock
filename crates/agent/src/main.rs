//! Windows User Agent の実行ファイル。
//!
//! LF-001 の範囲は「起動して自分の契約版を申告し、終了する」ところまでである。
//! named pipe の待受と単一起動の強制は LF-005、Reconciler と Operation は
//! LF-002 と LF-009 で追加する（`docs/14_BACKLOG.md`）。
//!
//! この段階では GitHub へ接続せず、Runner を登録・起動せず、WSL の状態を
//! 変更しない。

use std::io;

use runnerdock_protocol::{PROTOCOL_VERSION, Peer};

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

    let peer = Peer {
        protocol_version: PROTOCOL_VERSION,
        implementation: env!("CARGO_PKG_VERSION").to_owned(),
    };
    tracing::info!(
        protocol_version = peer.protocol_version,
        implementation = %peer.implementation,
        "runnerdock agent self-check"
    );

    // 起動確認を機械可読な形でも出す。ここに秘密は含めない。
    let line = serde_json::to_string(&peer).map_err(io::Error::other)?;
    println!("{line}");
    Ok(())
}
