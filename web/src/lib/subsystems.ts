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

export function subsystemKey(
  group: string | null | undefined,
  _role?: string | null,
): SubsystemKey {
  const g = group ? alias[group.toUpperCase()] : undefined;
  if (g) return g;
  return "OTHER";
}

export function subsystemTone(group: string | null | undefined, role?: string | null): Tone {
  return tone[subsystemKey(group, role)];
}
