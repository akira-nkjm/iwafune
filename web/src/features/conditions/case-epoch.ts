import type { CommandText } from "@/api/types";

/** ケースの開始時刻を書く（`fune case set-epoch|unset-epoch` と同じ引数。`POST /ops/from_command`）。時刻の読み方は service が持つ。空なら外す */
export function caseEpochCommand(caseId: string, epoch: string): CommandText {
  return { command: "case_epoch", case: caseId, epoch: epoch.trim() || null };
}
