import { getT } from "@/i18n";
import { isMat3, isVec3, type Json } from "./output";

/**
 * 解析の出力の欄の表示名（i18n の fields.labels）と出し方。出力は SI の数値なので、
 * **換算はせず**、単位と桁は POST /units/format（sat_units::format）に選ばせる。
 */

export function fieldLabel(key: string): string {
  return getT().fields.labels[key] ?? key;
}

/**
 * 欄の数の出し方。次元は解析の API が返す dims（JSON Pointer → 次元。比は `Fraction` で % になる）から引き、
 * 単位と桁は POST /units/format が選ぶ。dims に無い数は無次元として桁だけ整える。
 */
export interface FieldFormat {
  dimension: string;
}

export function fieldFormat(_key: string, dimension: string | null | undefined): FieldFormat {
  if (dimension) return { dimension };
  return { dimension: "Dimensionless" };
}

/** 数以外の値（文字・真偽・ベクトル・行列）の表示。数は FieldValue がサーバーに整形させる */
export function fieldText(value: Json | undefined): string {
  if (value === null || value === undefined) return "—";
  if (typeof value === "boolean") return value ? getT().fields.yes : getT().fields.no;
  if (typeof value === "string" || typeof value === "number") return String(value);
  if (isVec3(value)) return `(${value.x}, ${value.y}, ${value.z})`;
  if (isMat3(value)) return `[${value.m.map((r) => r.join(", ")).join("; ")}]`;
  if (Array.isArray(value)) return value.map((v) => fieldText(v)).join(", ");
  return JSON.stringify(value);
}
