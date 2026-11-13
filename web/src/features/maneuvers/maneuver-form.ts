import type { CommandText, DesignView, ManeuverText } from "@/api/types";

/** 新しいマヌーバの欄の初期値（区分と計算の段階は選択肢の先頭。値を読むのは service） */
export function emptyManeuverText(): ManeuverText {
  return { id: "", category: "insertion", method: "impulsive" };
}

/**
 * 欄に打った文字列 → `fune maneuver add|set` と同じ引数（`POST /ops/from_command`）。ID が空なら null。
 * 量・真偽を読むのも、空の欄を書かないのも service（ここでは文字列のまま渡す）
 */
export function maneuverCommand(text: ManeuverText, adding: boolean): CommandText | null {
  if (!text.id) return null;
  return adding ? { command: "maneuver_add", ...text } : { command: "maneuver_set", ...text };
}

/** スラスタに選べる部品（役割が推進系のスラスタか RCS のスラスタ。API の `PartRow.role` で選ぶ） */
export function thrusterOptions(design: DesignView): { value: string; label: string }[] {
  return design.parts
    .filter((p) => p.role === "propulsion_thruster" || p.role === "thruster")
    .map((p) => ({ value: p.id, label: p.id }));
}
