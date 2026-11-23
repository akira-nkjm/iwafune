import type { CommandText } from "@/api/types";

/** ケースのフェーズを書く（`fune case set-phase|unset-phase` と同じ引数。`POST /ops/from_command`）。フェーズが空なら外す */
export function casePhaseCommand(caseId: string, phase: string): CommandText {
  return { command: "case_phase", case: caseId, phase: phase || null };
}
