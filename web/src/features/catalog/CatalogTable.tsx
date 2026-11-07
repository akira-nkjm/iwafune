import { useEnumLabel } from "@/api/labels";
import type { CatalogItemView } from "@/api/types";
import { useT } from "@/i18n";
import { DataTable } from "@/kit/DataTable";
import { Pill } from "@/kit/Pill";
import { Td } from "@/kit/Td";
import { Th } from "@/kit/Th";
import { detailRowProps } from "@/kit/use-detail-route";
import { cn } from "@/lib/cn";
import { itemLabel } from "./catalog";

/** カタログ品の一覧（密な表）。行を選ぶと右に詳細。共有カタログの品にはカタログ名の札を付ける */
export function CatalogTable({
  items,
  selected,
  onSelect,
}: {
  items: CatalogItemView[];
  selected: string | null;
  onSelect: (id: string) => void;
}) {
  const t = useT();
  const label = useEnumLabel();
  return (
    <DataTable
      stickyHead
      className="h-full"
      head={
        <>
          <Th>{t.catalog.colId}</Th>
          <Th>{t.catalog.colKind}</Th>
          <Th>{t.catalog.colLabel}</Th>
          <Th>{t.catalog.colMaker}</Th>
          <Th>{t.catalog.colPartNo}</Th>
          <Th>{t.catalog.colMaturity}</Th>
          <Th align="right">{t.catalog.colUsed}</Th>
        </>
      }
    >
      {items.map((c) => (
        <tr
          key={c.id}
          {...detailRowProps(c.id, onSelect)}
          aria-selected={selected === c.id}
          className={cn(
            "cursor-pointer hover:bg-panel-2",
            selected === c.id && "bg-accent-tint hover:bg-accent-tint",
          )}
        >
          <Td mono>
            <span className="flex items-center gap-2">
              {c.id}
              {!c.editable && (
                <Pill dot={false} title={t.catalog.readOnly}>
                  {c.scope}
                </Pill>
              )}
            </span>
          </Td>
          <Td mono className="text-ink-2">
            {c.kind}
          </Td>
          <Td>{itemLabel(c)}</Td>
          <Td className="text-ink-2">{c.manufacturer ?? "—"}</Td>
          <Td mono className="text-ink-3">
            {c.part_number ?? "—"}
          </Td>
          <Td className="text-ink-3">{c.maturity ? label("maturity", c.maturity) : "—"}</Td>
          <Td align="right">{c.users.length}</Td>
        </tr>
      ))}
    </DataTable>
  );
}
