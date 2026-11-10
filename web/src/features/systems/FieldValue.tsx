import type { AnalysisLabel } from "@/api/types";
import { localized } from "@/i18n";
import { Formatted } from "@/quantity/Formatted";
import { fieldFormat, fieldText } from "./fields";
import { isVec3, type Json } from "./output";

/**
 * 解析の出力の 1 つの値。次元は API の dims（JSON Pointer → 次元）から受け取り、単位と桁は POST /units/format
 * （まとめて 1 回）が選ぶ。ベクトルは成分ごと。語彙の値（列挙）は API の labels（JSON Pointer → 表示名）で出す。
 * 数以外はそのまま。
 */
export function FieldValue({
  value,
  dimension,
  label,
}: {
  /** 欄の名前（出し方は次元だけで決まるので使わない。呼び手から外すのは解析の表示を共通の場所へ移すとき） */
  field: string;
  value: Json | undefined;
  dimension?: string | null;
  label?: AnalysisLabel | null;
}) {
  const f = fieldFormat(dimension);
  const one = (n: number) => <Formatted si={n} dimension={f.dimension} fallback={String(n)} />;
  if (typeof value === "number") return one(value);
  if (isVec3(value) && dimension)
    return (
      <>
        ({one(value.x)}, {one(value.y)}, {one(value.z)})
      </>
    );
  if (typeof value === "string" && label) return <>{localized(label.label, label.label_en)}</>;
  return <>{fieldText(value)}</>;
}
