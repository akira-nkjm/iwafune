import { Plus, Undo2, X } from "lucide-react";
import { useState } from "react";
import type { ThermalCoatingRow, ThermalSurfaceRow } from "@/api/types";
import { useT } from "@/i18n";
import { Empty } from "@/kit/feedback/Empty";
import { NativeSelect } from "@/kit/forms/NativeSelect";
import { TextInput } from "@/kit/forms/TextInput";
import { DataTable } from "@/kit/table/DataTable";
import { TablePanel } from "@/kit/table/TablePanel";
import { Td } from "@/kit/table/Td";
import { Th } from "@/kit/table/Th";
import { EditableQuantity } from "@/quantity/EditableQuantity";
import { quantityInputText } from "@/quantity/format";
import { Button } from "@/ui/button";
import { type ThermalDraft, withCell } from "./draft";

/** 面に表面材を足す欄（表面材は API の一覧から選び、割合は打った文字列のまま下書きに入れる） */
function AddFraction({
  row,
  coatings,
  draft,
  onDraft,
}: {
  row: ThermalSurfaceRow;
  coatings: ThermalCoatingRow[];
  draft: ThermalDraft;
  onDraft: (next: ThermalDraft) => void;
}) {
  const t = useT().systems.thermalNet;
  const [coating, setCoating] = useState("");
  const [text, setText] = useState("");
  const taken = new Set(row.fractions.map((f) => f.coating));
  const options = coatings
    .filter((c) => !taken.has(c.id))
    .map((c) => ({ value: c.id, label: c.id }));
  if (options.length === 0) return null;
  const add = () => {
    onDraft(withCell(draft, { cell: "surface", part: row.part, coating, fraction: text.trim() }));
    setCoating("");
    setText("");
  };
  return (
    <span className="inline-flex items-center gap-1">
      <NativeSelect
        size="sm"
        aria-label={`${t.addFraction}: ${row.part}`}
        value={coating}
        placeholder={t.addFraction}
        options={options}
        onChange={(e) => setCoating(e.target.value)}
      />
      <TextInput
        size="sm"
        mono
        aria-label={`${t.addFraction}: ${row.part}`}
        value={text}
        placeholder="0.5"
        className="w-16"
        onChange={(e) => setText(e.target.value)}
        onKeyDown={(e) => {
          if (e.key === "Enter" && coating && text.trim()) add();
        }}
      />
      <Button
        size="icon-xs"
        variant="outline"
        aria-label={`${t.addFraction}: ${row.part}`}
        disabled={!coating || !text.trim()}
        onClick={add}
      >
        <Plus />
      </Button>
    </span>
  );
}

/**
 * 外面の表: 面（部品）ごとの表面材と面積の割合。割合を書くと部品の吸収率・放射率の代わりに使われ、すべて外すと部品の値に戻る。
 * 割合の合計と一様な値は API の表示のまま（合計が 1 でないときの注意は保存の帯の検査が出す）。
 */
export function SurfacesPanel({
  surfaces,
  coatings,
  draft,
  onDraft,
}: {
  surfaces: ThermalSurfaceRow[];
  coatings: ThermalCoatingRow[];
  draft: ThermalDraft;
  onDraft: (next: ThermalDraft) => void;
}) {
  const t = useT().systems.thermalNet;
  return (
    <TablePanel title={t.surfacesTitle} caption={t.surfacesCaption}>
      {surfaces.length === 0 ? (
        <Empty>{t.noSurfaces}</Empty>
      ) : (
        <DataTable
          head={
            <>
              <Th>{t.part}</Th>
              <Th>{t.node}</Th>
              <Th>{t.area}</Th>
              <Th>{t.optics}</Th>
              <Th>
                <span className="sr-only">{t.useUniform}</span>
              </Th>
            </>
          }
        >
          {surfaces.map((row) => (
            <tr key={row.part} className="align-top hover:bg-panel-2/60">
              <Td mono>{row.part}</Td>
              <Td mono className="text-ink-2">
                {row.node ?? t.none}
              </Td>
              <Td className="num text-ink-2">{row.area?.display ?? t.none}</Td>
              <Td>
                <div className="flex flex-wrap items-center gap-x-3 gap-y-1">
                  {row.fractions.length === 0 && row.uniform && (
                    <span className="text-ink-3 text-xs">
                      {t.uniform(row.uniform.absorptivity.display, row.uniform.emissivity.display)}
                    </span>
                  )}
                  {row.fractions.map((f) => (
                    <span key={f.coating} className="inline-flex items-center gap-1">
                      <span className="font-mono text-[12px] text-ink-2">{f.coating}</span>
                      <EditableQuantity
                        display={f.fraction.display}
                        initial={quantityInputText(f.fraction)}
                        dimension={f.fraction.dimension}
                        label={`${row.part} / ${f.coating}`}
                        onCommit={(text) =>
                          onDraft(
                            withCell(draft, {
                              cell: "surface",
                              part: row.part,
                              coating: f.coating,
                              fraction: text,
                            }),
                          )
                        }
                      />
                      <Button
                        variant="ghost"
                        size="icon-xs"
                        aria-label={t.removeFraction(f.coating)}
                        title={t.removeFraction(f.coating)}
                        onClick={() =>
                          onDraft(
                            withCell(draft, {
                              cell: "surface",
                              part: row.part,
                              coating: f.coating,
                              fraction: null,
                            }),
                          )
                        }
                      >
                        <X />
                      </Button>
                    </span>
                  ))}
                  {row.fractions.length > 0 && (
                    <span className="num text-ink-3 text-xs">{t.total(row.total.display)}</span>
                  )}
                  <AddFraction row={row} coatings={coatings} draft={draft} onDraft={onDraft} />
                </div>
              </Td>
              <Td className="text-right">
                {row.fractions.length > 0 && (
                  <Button
                    variant="ghost"
                    size="xs"
                    aria-label={`${t.useUniform}: ${row.part}`}
                    onClick={() =>
                      onDraft(withCell(draft, { cell: "surface_remove", part: row.part }))
                    }
                  >
                    <Undo2 />
                    {t.useUniform}
                  </Button>
                )}
              </Td>
            </tr>
          ))}
        </DataTable>
      )}
    </TablePanel>
  );
}
