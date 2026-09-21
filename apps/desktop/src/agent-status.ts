/**
 * Agent の状態を Tauri の型付き command から取る。
 *
 * UI は pipe 名も実行ファイルの path も持たない。渡せる引数が無いこと自体が
 * 境界で、OS シェルや DB への経路は UI 側に存在しない（AGENTS.md §5.1）。
 * 取得できなかったことは失敗として返し、既定値で埋めない。
 */
import { invoke } from "@tauri-apps/api/core";
import type { AgentStatus, ErrorPayload } from "@runnerdock/contracts";

import { hasTauriShell } from "./shell-peer";

/** 取得結果。成功と失敗を型で分け、「不明」を成功へ丸めない。 */
export type AgentStatusOutcome =
  | { readonly kind: "connected"; readonly status: AgentStatus }
  /** Tauri の shell 上で動いていない（ブラウザ起動）。 */
  | { readonly kind: "noShell" }
  /** shell は居るが Agent へ繋げない、または Agent が拒否した。 */
  | { readonly kind: "refused"; readonly error: ErrorPayload };

/** Tauri から返った値が `ErrorPayload` の形をしているか。 */
function isErrorPayload(value: unknown): value is ErrorPayload {
  return (
    typeof value === "object" &&
    value !== null &&
    "code" in value &&
    "messageKey" in value
  );
}

/**
 * Agent の状態を取る。待受が無ければ shell 側が Agent を起こす。
 */
export async function fetchAgentStatus(): Promise<AgentStatusOutcome> {
  if (!hasTauriShell()) {
    return { kind: "noShell" };
  }
  try {
    const status = await invoke<AgentStatus>("agent_status");
    return { kind: "connected", status };
  } catch (error: unknown) {
    if (isErrorPayload(error)) {
      return { kind: "refused", error };
    }
    // 型付きで返らなかった失敗も、成功にも既定値にも倒さない。
    throw error;
  }
}
