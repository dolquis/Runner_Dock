//! WSL Guest の実行ファイル。
//!
//! Guest は Windows Agent が `wsl.exe --exec` で起動する小さなプロセスで、
//! ヘルス照会、許可済み Runner の起動・停止、ログ転送だけを受け付ける
//! （`docs/03_ARCHITECTURE.md` §4）。汎用のリモートシェルは提供しない。
//!
//! LF-001 の範囲は「起動して自分の契約版を申告し、終了する」ところまでである。
//! 長寿命 stdio 接続と寿命管理の検証は LF-004 で行う。
//!
//! この実行ファイルは Linux 上で動くが、LF-001 では WSL 固有 API を使わない
//! ため開発ホストでもビルド・実行できる。

use std::io;

use runnerdock_protocol::PROTOCOL_MAJOR;
use runnerdock_protocol::dto::HandshakeRequest;

fn main() -> io::Result<()> {
    tracing_subscriber::fmt()
        // Guest の protocol は stdout を使う予定のため、診断ログは stderr へ出す
        // （`docs/14_BACKLOG.md` LF-004「protocol/stdout 分離」）。
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
        "runnerdock guest self-check"
    );

    let line = serde_json::to_string(&peer).map_err(io::Error::other)?;
    println!("{line}");
    Ok(())
}
