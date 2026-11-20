import type { CommandText, RequirementText } from "@/api/types";

/** 新しい要求の欄の初期値（制約の向きは選択肢の先頭。値を読むのは service） */
export function emptyRequirementText(): RequirementText {
  return { id: "", statement: "", quantity: "", kind: "at_most" };
}

/**
 * 欄に打った文字列 → `fune requirement add|set` と同じ引数（`POST /ops/from_command`）。ID が空なら null。
 * 量・真偽・ケースの列を読むのも、空の欄を書かないのも service（ここでは文字列のまま渡す）
 */
export function requirementCommand(text: RequirementText, adding: boolean): CommandText | null {
  if (!text.id) return null;
  return adding ? { command: "requirement_add", ...text } : { command: "requirement_set", ...text };
}
