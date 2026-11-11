import type { DesignView, PartRow } from "@/api/types";
import { useT } from "@/i18n";
import { SystemDot } from "@/kit/SystemDot";
import { DataTable } from "@/kit/table/DataTable";
import { SelectableRow } from "@/kit/table/SelectableRow";
import { Td } from "@/kit/table/Td";
import { Th } from "@/kit/table/Th";
import { attrOf, systemTitle } from "@/lib/design";
import { valueText } from "@/quantity/format";

/** 部品の一覧（密な表）。行を選ぶと右のパネルに詳細を出す */
export function PartsTable({
  design,
  items,
  selected,
  onSelect,
}: {
  design: DesignView;
  /** 絞り込んだ部品（出す行） */
  items: PartRow[];
  selected: string | null;
  onSelect: (id: string) => void;
}) {
  const t = useT().components.table;
  const diagsByPart = new Map<string, { errors: number; warns: number }>();
  for (const d of design.diagnostics) {
    const id = d.part;
    if (!id) continue;
    const c = diagsByPart.get(id) ?? { errors: 0, warns: 0 };
    if (d.severity === "error") c.errors += 1;
    else if (d.severity === "warning") c.warns += 1;
    diagsByPart.set(id, c);
  }
  return (
    <DataTable
      stickyHead
      className="h-full"
      head={
        <>
          <Th>{t.id}</Th>
          <Th>{t.group}</Th>
          <Th>{t.kind}</Th>
          <Th>{t.catalog}</Th>
          <Th align="right">{t.count}</Th>
          <Th align="right">{t.mass}</Th>
          <Th>{t.power}</Th>
          <Th align="right" title={t.unsetTitle}>
            {t.unset}
          </Th>
          <Th align="right">{t.diagnostics}</Th>
        </>
      }
    >
      {items.map((p) => {
        const mass = attrOf(p, "mass");
        const power = attrOf(p, "power");
        const unset = Object.values(p.values).filter((v) => v.origin === "unset").length;
        const { errors, warns } = diagsByPart.get(p.id) ?? { errors: 0, warns: 0 };
        return (
          <SelectableRow key={p.id} selected={selected === p.id} onSelect={() => onSelect(p.id)}>
            <Td>
              <span className="flex items-center gap-2">
                <SystemDot system={p.system} />
                <span className="font-medium font-mono text-[12.5px]">{p.id}</span>
              </span>
            </Td>
            <Td className="text-ink-2">{p.system ? systemTitle(p.system, design.defs) : "—"}</Td>
            <Td mono className="text-ink-2">
              {p.kind}
            </Td>
            <Td mono className="max-w-48 truncate text-ink-3">
              {p.catalog ?? "—"}
            </Td>
            <Td align="right">{p.count}</Td>
            <Td align="right">{mass ? valueText(mass) : "—"}</Td>
            <Td className="max-w-56 truncate text-ink-2">
              <span title={power ? valueText(power) : undefined}>
                {power ? valueText(power) : "—"}
              </span>
            </Td>
            <Td align="right" className={unset > 0 ? "text-warn-ink" : "text-ink-4"}>
              {unset}
            </Td>
            <Td align="right">
              {errors > 0 ? (
                <span className="text-neg-ink">{errors}</span>
              ) : warns > 0 ? (
                <span className="text-warn-ink">{warns}</span>
              ) : (
                <span className="text-ink-4">0</span>
              )}
            </Td>
          </SelectableRow>
        );
      })}
    </DataTable>
  );
}
