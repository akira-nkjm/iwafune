import type { System } from "@/api/types";
import type { Tone } from "./tone";

/** 系 → 色の唯一の対応。表示名・ディレクトリ名・役割から色を推測しない。 */
const tones: Record<System, Tone> = {
  aocs: "blue",
  comm: "gold",
  power: "green",
  cdh: "navy",
  payload: "rose",
  thermal: "orange",
  structure: "violet",
  propulsion: "magenta",
};

/** API が返す系の色。未所属は中立色。 */
export function systemTone(system: System | null | undefined): Tone {
  return system ? tones[system] : "gray";
}
