//! 型付き IPC クライアント `runnerdock`。
//!
//! LF-001 の範囲は版の表示だけである。`doctor` / `status` / `start` / `stop`
//! は GUI と同じ契約から実行する形で LF-016 で追加する。
//!
//! JSON 出力に秘密を含めない（`docs/14_BACKLOG.md` LF-016）。

use clap::{Parser, Subcommand};
use runnerdock_protocol::PROTOCOL_MAJOR;
use runnerdock_protocol::dto::BackendKind;
use serde::Serialize;

#[derive(Debug, Parser)]
#[command(name = "runnerdock", version, about = "Runner Dock CLI", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// CLI 実装版、ワイヤー契約版、既知の Backend 種別を表示する。
    Version {
        /// 機械可読な JSON で出力する。
        #[arg(long)]
        json: bool,
    },
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct VersionReport {
    implementation: &'static str,
    protocol_major: u32,
    backend_kinds: Vec<&'static str>,
}

impl VersionReport {
    fn collect() -> Self {
        Self {
            implementation: env!("CARGO_PKG_VERSION"),
            protocol_major: PROTOCOL_MAJOR,
            backend_kinds: BackendKind::ALL.iter().map(|kind| kind.as_str()).collect(),
        }
    }
}

fn main() -> Result<(), serde_json::Error> {
    let cli = Cli::parse();
    match cli.command {
        Command::Version { json } => {
            let report = VersionReport::collect();
            if json {
                println!("{}", serde_json::to_string(&report)?);
            } else {
                println!("implementation: {}", report.implementation);
                println!("protocol: {}", report.protocol_major);
                println!("backends: {}", report.backend_kinds.join(", "));
            }
        }
    }
    Ok(())
}
