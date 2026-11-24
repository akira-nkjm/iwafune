import { Undo2, X } from "lucide-react";
import type { ThermalCouplingRow, ThermalCouplingsTable } from "@/api/types";
import { localized, useT } from "@/i18n";
import { Empty } from "@/kit/feedback/Empty";
import { DataTable } from "@/kit/table/DataTable";
import { TablePanel } from "@/kit/table/TablePanel";
import { Td } from "@/kit/table/Td";
import { Th } from "@/kit/table/Th";
import { cn } from "@/lib/cn";
import { EditableQuantity } from "@/quantity/EditableQuantity";
import { quantityInputText } from "@/quantity/format";
import { Button } from "@/ui/button";
import { AddCoupling } from "./AddCoupling";
import { cellKey, type ThermalDraft, withCell, withoutCell } from "./draft";

/** 行の宛先（書き換え・取り消しのセルの宛先。値は付けない） */
function target(row: ThermalCouplingRow) {
  return { cell: "coupling", kind: row.kind, a: row.a, b: row.b } as const;
}

/**
 * 節点間の結合の表: 解析が使っている値と出どころ（API の値をそのまま並べる）。値を打つと、その対の明示の結合を書く下書きになり、
 * 書いた値は取り消せる（既定の出どころへ戻る）。足す対は下の欄で選ぶ。両端・値・出どころの扱いは service が決める。
 */
export function CouplingsPanel({
  table,
  draft,
  onDraft,
}: {
  table: ThermalCouplingsTable;
  draft: ThermalDraft;
  onDraft: (next: ThermalDraft) => void;
}) {
  const t = useT().systems.thermalNet;
  const kindLabel = (row: ThermalCouplingRow) => {
    const k = table.kinds.find((x) => x.kind === row.kind);
    return k ? localized(k.label.label, k.label.label_en) : row.kind;
  };
  return (
    <TablePanel title={t.couplingsTitle} caption={t.couplingsCaption}>
      {!table.available ? (
        <Empty>{t.unavailable}</Empty>
      ) : (
        <>
          <DataTable
            head={
              <>
                <Th>{t.kind}</Th>
                <Th>{t.nodeA}</Th>
                <Th>{t.nodeB}</Th>
                <Th>{t.value}</Th>
                <Th>{t.source}</Th>
                <Th>
                  <span className="sr-only">{t.withdraw}</span>
                </Th>
              </>
            }
          >
            {table.rows.map((row) => {
              const key = cellKey({ ...target(row), value: null });
              const changed = draft.has(key);
              return (
                <tr
                  key={key}
                  data-changed={changed || undefined}
                  className={cn("hover:bg-panel-2/60", changed && "bg-accent-tint/40")}
                >
                  <Td className="text-ink-2">{kindLabel(row)}</Td>
                  <Td mono>{row.a}</Td>
                  <Td mono>{row.b}</Td>
                  <Td>
                    <EditableQuantity
                      display={row.value.display}
                      initial={quantityInputText(row.value)}
                      dimension={row.value.dimension}
                      label={`${row.a} – ${row.b} (${kindLabel(row)})`}
                      muted={!row.explicit}
                      onCommit={(text) => onDraft(withCell(draft, { ...target(row), value: text }))}
                    />
                  </Td>
                  <Td className="text-ink-2">
                    <span>{localized(row.source_label.label, row.source_label.label_en)}</span>
                    {row.contact && (
                      <span className="ml-2 text-[11px] text-ink-3">
                        {t.contact(
                          row.contact.area.display,
                          row.contact.mounts,
                          row.contact.fixed_mounts,
                        )}
                      </span>
                    )}
                  </Td>
                  <Td className="whitespace-nowrap text-right">
                    {changed && (
                      <Button
                        variant="ghost"
                        size="icon-xs"
                        aria-label={t.undo}
                        title={t.undo}
                        onClick={() => onDraft(withoutCell(draft, key))}
                      >
                        <Undo2 />
                      </Button>
                    )}
                    {row.explicit && (
                      <Button
                        variant="ghost"
                        size="xs"
                        title={t.withdrawHint}
                        aria-label={`${t.withdraw}: ${row.a} – ${row.b}`}
                        onClick={() => onDraft(withCell(draft, { ...target(row), value: null }))}
                      >
                        <X />
                        {t.withdraw}
                      </Button>
                    )}
                  </Td>
                </tr>
              );
            })}
          </DataTable>
          <AddCoupling table={table} draft={draft} onDraft={onDraft} />
        </>
      )}
    </TablePanel>
  );
}
