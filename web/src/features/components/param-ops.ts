import type { CommandText } from "@/api/types";
import type { NewAttribute } from "./NewAttributeFields";

/**
 * 新しく定義する属性の欄 → `fune attr add` と同じ引数（`CommandText` の attr_add）。ID・次元が無ければ null。
 * 定義ページの属性の作成と「パラメーター追加」で共通。Op（create_attribute）にするのは service（`useCommandOps`）で、
 * 表示名の既定（空なら ID）・値の型の既定（scalar）も service が決める
 */
export function attrAddCommand(fresh: NewAttribute): CommandText | null {
  if (!fresh.id || !fresh.dimension) return null;
  return {
    command: "attr_add",
    id: fresh.id,
    label: fresh.label,
    dimension: fresh.dimension,
    units: fresh.units,
    by_state: fresh.byState,
    symbols: [],
    unit_vector: false,
  };
}
