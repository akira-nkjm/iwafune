import type { Tone } from "./tone";

/**
 * サブシステムの色。電源（POWER）は姿勢（AOCS）の青と見分けやすい緑、熱（THM）は橙、構造（STR）は紫。
 * 呼び名は AOCS・POWER・CDH で、ADCS・EPS などは別名として受ける。
 * 表・系統図・3D・積み上げ棒で同じ対応を使う。Group の ID は表記揺れがあるので正規化して引く。
 */
export type SubsystemKey =
  | "AOCS"
  | "COM"
  | "POWER"
  | "CDH"
  | "PROP"
  | "PL"
  | "THM"
  | "STR"
  | "OTHER";

const tone: Record<SubsystemKey, Tone> = {
  AOCS: "blue",
  COM: "gold",
  POWER: "green",
  CDH: "navy",
  PROP: "red",
  PL: "rose",
  THM: "orange",
  STR: "violet",
  OTHER: "gray",
};

const alias: Record<string, SubsystemKey> = {
  AOCS: "AOCS",
  ADCS: "AOCS",
  ATTITUDE: "AOCS",
  COM: "COM",
  COMM: "COM",
  COMMS: "COM",
  TTC: "COM",
  POWER: "POWER",
  EPS: "POWER",
  CDH: "CDH",
  "C&DH": "CDH",
  OBC: "CDH",
  OBDH: "CDH",
  PROP: "PROP",
  PROPULSION: "PROP",
  PL: "PL",
  PAYLOAD: "PL",
  MISSION: "PL",
  THM: "THM",
  THERMAL: "THM",
  TCS: "THM",
  STR: "STR",
  STRUCTURE: "STR",
  STRUCT: "STR",
};

/** Group の ID → サブシステム（表記揺れを正規化する。知らない ID と、Group の無い部品は OTHER） */
export function subsystemKey(group: string | null | undefined): SubsystemKey {
  const g = group ? alias[group.toUpperCase()] : undefined;
  if (g) return g;
  return "OTHER";
}

/** サブシステムの色（`subsystemKey` の対応） */
export function subsystemTone(group: string | null | undefined): Tone {
  return tone[subsystemKey(group)];
}
