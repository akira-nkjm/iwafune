import type { Op } from "@/api/types";
import type { NewAttribute } from "./NewAttributeFields";

/**
 * 「パラメーター追加」で新しく定義する属性の Op（create_attribute。値の型はスカラー）。ID・次元が無ければ null。
 * 値の Op は組み立てない: 打った文字列をこの定義を当てた設計で service が読む（`useTextOp` の `ops`）
 */
export function createAttributeOp(fresh: NewAttribute): Op | null {
  if (!fresh.id || !fresh.dimension) return null;
  return {
    op: "create_attribute",
    def: {
      id: fresh.id,
      label: fresh.label || fresh.id,
      type: "scalar",
      dimension: fresh.dimension,
      units: fresh.units,
      by_state: fresh.byState,
    },
  };
}
