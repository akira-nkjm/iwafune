import { Formatted } from "@/quantity/Formatted";
import { fieldFormat, fieldText } from "./fields";
import { isVec3, type Json } from "./output";

/**
 * 解析の出力の 1 つの値。次元は API の dims（JSON Pointer → 次元）から受け取り、単位と桁は POST /units/format
 * （まとめて 1 回）が選ぶ。ベクトルは成分ごと。数以外はそのまま。
 */
export function FieldValue({
  field,
  value,
  dimension,
}: {
  field: string;
  value: Json | undefined;
  dimension?: string | null;
}) {
  const f = fieldFormat(field, dimension);
  const one = (n: number) => <Formatted si={n} dimension={f.dimension} fallback={String(n)} />;
  if (typeof value === "number") return one(value);
  if (isVec3(value) && dimension)
    return (
      <>
        ({one(value.x)}, {one(value.y)}, {one(value.z)})
      </>
    );
  return <>{fieldText(value)}</>;
}
