/**
 * Desktop shell（Tauri の Rust 側）の自己申告を取る。
 *
 * ブラウザだけで動かす mock 起動経路では Tauri の IPC が無いため、取得できない
 * ことを `null` として区別する。取得できなかったことを既定値で埋めない。
 */
import { invoke } from "@tauri-apps/api/core";

/** `crates/protocol` の `Peer` に対応する型。LF-002 で生成型へ置き換える。 */
export interface ShellPeer {
  protocolVersion: number;
  implementation: string;
}

/** Tauri の shell 上で動いているかを判定する。 */
export function hasTauriShell(): boolean {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}

/**
 * shell の自己申告を取得する。Tauri 上で動いていない場合は `null` を返す。
 */
export async function fetchShellPeer(): Promise<ShellPeer | null> {
  if (!hasTauriShell()) {
    return null;
  }
  return await invoke<ShellPeer>("shell_peer");
}
