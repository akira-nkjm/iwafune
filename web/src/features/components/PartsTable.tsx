import type { DesignView, PartRow } from "@/api/types";
import { useT } from "@/i18n";
import { DataTable } from "@/kit/DataTable";
import { SubsystemDot } from "@/kit/SubsystemDot";
import { Td } from "@/kit/Td";
import { Th } from "@/kit/Th";
import { detailRowProps } from "@/kit/use-detail-route";
import { cn } from "@/lib/cn";
import { attrOf, nodeName } from "@/lib/design";
import { valueText } from "@/quantity/format";

/** 部品の一覧（密な表）。行を選ぶと右のパネルに詳細を出す */
export function PartsTable({
  design,
  parts,
  selected,
  onSelect,
}: {
  design: DesignView;
  parts: PartRow[];
  selected: string | null;
  onSelect: (id: string) => void;
}) {
  const t = useT().components.table;
  const label = new Map(design.tree.map((g) => [g.id, nodeName(g)]));
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
      {parts.map((p) => {
        const mass = attrOf(p, "mass");
        const power = attrOf(p, "power");
        const unset = Object.values(p.values).filter((v) => v.origin === "unset").length;
        const { errors, warns } = diagsByPart.get(p.id) ?? { errors: 0, warns: 0 };
        return (
          <tr
            key={p.id}
            {...detailRowProps(p.id, onSelect)}
            aria-selected={selected === p.id}
            className={cn(
              "cursor-pointer hover:bg-panel-2",
              selected === p.id && "bg-accent-tint hover:bg-accent-tint",
            )}
          >
            <Td>
              <span className="flex items-center gap-2">
                <SubsystemDot group={p.subsystem} role={p.role} />
                <span className="font-medium font-mono text-[12.5px]">{p.id}</span>
              </span>
            </Td>
            <Td className="text-ink-2">{p.parent ? (label.get(p.parent) ?? p.parent) : "—"}</Td>
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
          </tr>
        );
      })}
    </DataTable>
  );
}
