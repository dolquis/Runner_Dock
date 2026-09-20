/**
 * Desktop shell（Tauri の Rust 側）の自己申告を取る。
 *
 * ブラウザだけで動かす mock 起動経路では Tauri の IPC が無いため、取得できない
 * ことを `null` として区別する。取得できなかったことを既定値で埋めない。
 */
import { invoke } from "@tauri-apps/api/core";
import type { HandshakeRequest } from "@runnerdock/contracts";

/**
 * shell の自己申告。`crates/protocol` から生成した型をそのまま使う。
 *
 * 手で写した型を置かない。契約が変われば生成物が変わり、`pnpm lint` の型検査と
 * `contracts-gen --check` の両方で食い違いが出る（ADR-017）。
 */
export type ShellPeer = HandshakeRequest;

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
