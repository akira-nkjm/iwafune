import type { CommandText, PhaseText } from "@/api/types";

/** 新しいフェーズの欄の初期値（値を読むのは service） */
export function emptyPhaseText(): PhaseText {
  return { id: "" };
}

/**
 * 欄に打った文字列 → `fune phase add|set` と同じ引数（`POST /ops/from_command`）。ID が空なら null。
 * 量・整数を読むのも、空の欄を書かないのも service（ここでは文字列のまま渡す）
 */
export function phaseCommand(text: PhaseText, adding: boolean): CommandText | null {
  if (!text.id) return null;
  return adding ? { command: "phase_add", ...text } : { command: "phase_set", ...text };
}
