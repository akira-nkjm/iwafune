import type { DesignView, PartRow } from "@/api/types";
import { tableComparator } from "@/kit/table/useTableSort";
import { attrOf } from "@/lib/design";
import { valueSortSi } from "@/quantity/sort";

/** 診断列の表示とソートで共有する部品ごとの件数。 */
export function partDiagnostics(design: DesignView) {
  const counts = new Map<string, { errors: number; warns: number }>();
  for (const d of design.diagnostics) {
    if (!d.part) continue;
    const c = counts.get(d.part) ?? { errors: 0, warns: 0 };
    if (d.severity === "error") c.errors += 1;
    else if (d.severity === "warning") c.warns += 1;
    counts.set(d.part, c);
  }
  return counts;
}

export function unsetCount(p: PartRow): number {
  return Object.values(p.values).filter((v) => v.origin === "unset").length;
}

const numeric = (a: number, b: number) => a - b;
const natural = (a: string, b: string) =>
  a.localeCompare(b, undefined, { numeric: true, sensitivity: "base" });

/** 部品表の列定義。系の並びは API の定義順で、量は表示文字列を解釈せず比較する。 */
export function partsComparators(design: DesignView, diags = partDiagnostics(design)) {
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
    unset: column(unsetCount, numeric),
    diagnostics: column(
      (p) => diags.get(p.id) ?? { errors: 0, warns: 0 },
      (a, b) => a.errors - b.errors || a.warns - b.warns,
    ),
  };
}
