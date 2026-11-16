import type { DesignView, PartRow } from "@/api/types";
import { tableComparator } from "@/kit/table/useTableSort";
import { attrOf } from "@/lib/design";
import { valueSortSi } from "@/quantity/sort";

const numeric = (a: number, b: number) => a - b;
const natural = (a: string, b: string) =>
  a.localeCompare(b, undefined, { numeric: true, sensitivity: "base" });

/** 部品表の列定義。系の並びは API の定義順で、量は表示文字列を解釈せず比較する。診断の件数と未設定の数は API の `PartRow` の値。 */
export function partsComparators(design: DesignView) {
  const column = <Value>(
    value: (p: PartRow) => Value | null | undefined,
    compare: (a: Value, b: Value) => number,
  ) => tableComparator(value, compare);
  return {
    id: column((p) => p.id, natural),
    group: column((p) => (p.system ? design.defs.systems.indexOf(p.system) : null), numeric),
    kind: column((p) => p.kind, natural),
    catalog: column((p) => p.catalog, natural),
    count: column((p) => p.count, numeric),
    mass: column((p) => valueSortSi(attrOf(p, "mass")?.value), numeric),
    power: column((p) => valueSortSi(attrOf(p, "power")?.value), numeric),
    unset: column((p) => p.unset_count, numeric),
    diagnostics: column(
      (p) => p.diagnostics,
      (a, b) => a.errors - b.errors || a.warnings - b.warnings,
    ),
  };
}
