import { useEnumLabel } from "@/api/labels";
import type { CatalogItemView, DesignView } from "@/api/types";
import { useT } from "@/i18n";
import { Pill } from "@/kit/status/Pill";
import { DataTable } from "@/kit/table/DataTable";
import { SelectableRow } from "@/kit/table/SelectableRow";
import { Td } from "@/kit/table/Td";
import { Th } from "@/kit/table/Th";
import { useTableSort } from "@/kit/table/useTableSort";
import { kindLabel } from "@/lib/design";
import { itemLabel } from "./catalog";
import { catalogComparators } from "./catalog-sort";

/** カタログ品の一覧（密な表。見出しで並べ替え）。行を選ぶと詳細。共有カタログの品はカタログ名を札にする */
export function CatalogTable({
  design,
  items,
  selected,
  onSelect,
}: {
  design: DesignView;
  items: CatalogItemView[];
  selected: string | null;
  onSelect: (id: string) => void;
}) {
  const t = useT().catalog;
  const label = useEnumLabel();
  const { rows, headerProps } = useTableSort(
    items,
    catalogComparators(design.defs, (v) => label("maturity", v)),
  );
  return (
    <DataTable
      stickyHead
      className="h-full"
      head={
        <>
          <Th {...headerProps("id")}>{t.colId}</Th>
          <Th {...headerProps("catalog")}>{t.colCatalog}</Th>
          <Th {...headerProps("kind")}>{t.colKind}</Th>
          <Th {...headerProps("label")}>{t.colLabel}</Th>
          <Th {...headerProps("maker")}>{t.colMaker}</Th>
          <Th {...headerProps("partNo")}>{t.colPartNo}</Th>
          <Th {...headerProps("maturity")}>{t.colMaturity}</Th>
          <Th align="right" {...headerProps("used")}>
            {t.colUsed}
          </Th>
        </>
      }
    >
      {rows.map((c) => (
        <SelectableRow key={c.id} selected={selected === c.id} onSelect={() => onSelect(c.id)}>
          <Td mono>{c.id}</Td>
          <Td className="text-ink-2">
            {c.editable ? (
              c.scope
            ) : (
              <Pill dot={false} title={t.readOnly(c.scope)}>
                {c.scope}
              </Pill>
            )}
          </Td>
          <Td className="text-ink-2">
            <span title={c.kind}>{kindLabel(design.defs, c.kind)}</span>
          </Td>
          <Td>{itemLabel(c)}</Td>
          <Td className="text-ink-2">{c.manufacturer ?? "—"}</Td>
          <Td mono className="text-ink-3">
            {c.part_number ?? "—"}
          </Td>
          <Td className="text-ink-3">{c.maturity ? label("maturity", c.maturity) : "—"}</Td>
          <Td align="right">{c.users.length}</Td>
        </SelectableRow>
      ))}
    </DataTable>
  );
}
