import type { DesignView, PartRow } from "@/api/types";
import { useT } from "@/i18n";
import { SystemDot } from "@/kit/SystemDot";
import { DataTable } from "@/kit/table/DataTable";
import { SelectableRow } from "@/kit/table/SelectableRow";
import { Td } from "@/kit/table/Td";
import { Th } from "@/kit/table/Th";
import { type SortHeaderProps, useTableSort } from "@/kit/table/useTableSort";
import { attrOf, systemTitle } from "@/lib/design";
import { valueText } from "@/quantity/format";
import { partsComparators } from "./parts-sort";

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
  const { rows, headerProps } = useTableSort(items, partsComparators(design));
  return (
    <DataTable stickyHead className="h-full" head={<PartsTableHead headerProps={headerProps} />}>
      {rows.map((p) => (
        <PartTableRow
          key={p.id}
          p={p}
          design={design}
          selected={selected === p.id}
          onSelect={() => onSelect(p.id)}
        />
      ))}
    </DataTable>
  );
}

function PartTableRow({
  p,
  design,
  selected,
  onSelect,
}: {
  p: PartRow;
  design: DesignView;
  selected: boolean;
  onSelect: () => void;
}) {
  const mass = attrOf(p, "mass");
  const power = attrOf(p, "power");
  const unset = p.unset_count;
  return (
    <SelectableRow key={p.id} selected={selected} onSelect={onSelect}>
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
        <span title={power ? valueText(power) : undefined}>{power ? valueText(power) : "—"}</span>
      </Td>
      <Td align="right" className={unset > 0 ? "text-warn-ink" : "text-ink-4"}>
        {unset}
      </Td>
      <Td align="right">
        <DiagnosticCount errors={p.diagnostics.errors} warns={p.diagnostics.warnings} />
      </Td>
    </SelectableRow>
  );
}

function PartsTableHead({
  headerProps,
}: {
  headerProps: (column: keyof ReturnType<typeof partsComparators>) => { sort: SortHeaderProps };
}) {
  const t = useT().components.table;
  return (
    <>
      <Th {...headerProps("id")}>{t.id}</Th>
      <Th {...headerProps("group")}>{t.group}</Th>
      <Th {...headerProps("kind")}>{t.kind}</Th>
      <Th {...headerProps("catalog")}>{t.catalog}</Th>
      <Th {...headerProps("count")} align="right">
        {t.count}
      </Th>
      <Th {...headerProps("mass")} align="right">
        {t.mass}
      </Th>
      <Th {...headerProps("power")}>{t.power}</Th>
      <Th {...headerProps("unset")} align="right" title={t.unsetTitle}>
        {t.unset}
      </Th>
      <Th {...headerProps("diagnostics")} align="right">
        {t.diagnostics}
      </Th>
    </>
  );
}

function DiagnosticCount({ errors, warns }: { errors: number; warns: number }) {
  if (errors > 0) return <span className="text-neg-ink">{errors}</span>;
  if (warns > 0) return <span className="text-warn-ink">{warns}</span>;
  return <span className="text-ink-4">0</span>;
}
