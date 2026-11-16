import type { CatalogItemView, DefsView } from "@/api/types";
import { tableComparator } from "@/kit/table/useTableSort";
import { kindLabel } from "@/lib/design";
import { itemLabel } from "./catalog";

const numeric = (a: number, b: number) => a - b;
const natural = (a: string, b: string) =>
  a.localeCompare(b, undefined, { numeric: true, sensitivity: "base" });

/**
 * カタログ品の表の列定義。種別は表示名で、成熟度は表示名（`maturityLabel`）で並べる。
 * 値の無い品はどちらの向きでも末尾（`tableComparator`）。
 */
export function catalogComparators(
  defs: Pick<DefsView, "kinds">,
  maturityLabel: (value: string) => string,
) {
  const column = <Value>(
    value: (c: CatalogItemView) => Value | null | undefined,
    compare: (a: Value, b: Value) => number,
  ) => tableComparator(value, compare);
  return {
    id: column((c) => c.id, natural),
    catalog: column((c) => c.scope, natural),
    kind: column((c) => kindLabel(defs, c.kind), natural),
    label: column((c) => itemLabel(c), natural),
    maker: column((c) => c.manufacturer, natural),
    partNo: column((c) => c.part_number, natural),
    maturity: column((c) => (c.maturity ? maturityLabel(c.maturity) : null), natural),
    used: column((c) => c.users.length, numeric),
  };
}
