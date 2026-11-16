import { useEnumLabel } from "@/api/labels";
import type { CatalogItemView } from "@/api/types";
import { useT } from "@/i18n";
import { Pill } from "@/kit/status/Pill";
import { DataTable } from "@/kit/table/DataTable";
import { SelectableRow } from "@/kit/table/SelectableRow";
import { Td } from "@/kit/table/Td";
import { Th } from "@/kit/table/Th";
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
          <Th>{t.catalog.colTags}</Th>
          <Th align="right">{t.catalog.colUsed}</Th>
        </>
      }
    >
      {items.map((c) => (
        <SelectableRow key={c.id} selected={selected === c.id} onSelect={() => onSelect(c.id)}>
          <Td mono>
            <span className="flex items-center gap-2">
              {c.id}
              {!c.editable && (
                <Pill dot={false} title={t.catalog.readOnly(c.scope)}>
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
          <Td>
            {c.tags.length > 0 ? (
              <span className="flex flex-wrap gap-1">
                {c.tags.map((tag) => (
                  <Pill key={tag} dot={false}>
                    {tag}
                  </Pill>
                ))}
              </span>
            ) : (
              "—"
            )}
          </Td>
          <Td align="right">{c.users.length}</Td>
        </SelectableRow>
      ))}
    </DataTable>
  );
}
