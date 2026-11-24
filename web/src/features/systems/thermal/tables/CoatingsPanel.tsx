import { Plus, Trash2 } from "lucide-react";
import { useState } from "react";
import type { ThermalCoatingRow } from "@/api/types";
import { useT } from "@/i18n";
import { Empty } from "@/kit/feedback/Empty";
import { CommitInput } from "@/kit/forms/CommitInput";
import { TextInput } from "@/kit/forms/TextInput";
import { DataTable } from "@/kit/table/DataTable";
import { TablePanel } from "@/kit/table/TablePanel";
import { Td } from "@/kit/table/Td";
import { Th } from "@/kit/table/Th";
import { EditableQuantity } from "@/quantity/EditableQuantity";
import { quantityInputText } from "@/quantity/format";
import { Button } from "@/ui/button";
import { coatingCell, type ThermalDraft, withCell } from "./draft";

type Optics = "absorptivity" | "emissivity";

/** 表面材を足す欄（ID・ラベル・α・ε を打った文字列のまま下書きに入れる。数として読むのは service） */
function AddCoating({
  draft,
  onDraft,
}: {
  draft: ThermalDraft;
  onDraft: (next: ThermalDraft) => void;
}) {
  const t = useT().systems.thermalNet;
  const [id, setId] = useState("");
  const [label, setLabel] = useState("");
  const [alpha, setAlpha] = useState("");
  const [epsilon, setEpsilon] = useState("");
  const ready = id.trim() !== "" && alpha.trim() !== "" && epsilon.trim() !== "";
  const add = () => {
    const key = id.trim();
    let next = label.trim() ? withCell(draft, coatingCell(key, "label", label.trim())) : draft;
    next = withCell(next, coatingCell(key, "absorptivity", alpha.trim()));
    next = withCell(next, coatingCell(key, "emissivity", epsilon.trim()));
    onDraft(next);
    setId("");
    setLabel("");
    setAlpha("");
    setEpsilon("");
  };
  const field = (
    name: string,
    value: string,
    set: (v: string) => void,
    className: string,
    mono = true,
  ) => (
    <TextInput
      size="sm"
      mono={mono}
      aria-label={`${t.addCoatingTitle}: ${name}`}
      placeholder={name}
      value={value}
      className={className}
      onChange={(e) => set(e.target.value)}
    />
  );
  return (
    <fieldset className="flex flex-wrap items-center gap-2 border-line border-t p-3">
      <legend className="sr-only">{t.addCoatingTitle}</legend>
      <span className="font-medium text-ink-2 text-xs">{t.addCoatingTitle}</span>
      {field(t.id, id, setId, "w-32")}
      {field(t.label, label, setLabel, "w-40", false)}
      {field(t.absorptivity, alpha, setAlpha, "w-28")}
      {field(t.emissivity, epsilon, setEpsilon, "w-28")}
      <Button size="sm" variant="outline" disabled={!ready} onClick={add}>
        <Plus />
        {t.addCoating}
      </Button>
    </fieldset>
  );
}

/**
 * 表面材の表: ラベルと吸収率 α・放射率 ε を直接書き換え、足し、消す。面が使っている表面材を消す下書きは、
 * service の検査（`ref.coating`）が保存の帯で弾く（使っている面は API の `used_by`）。
 */
export function CoatingsPanel({
  coatings,
  draft,
  onDraft,
}: {
  coatings: ThermalCoatingRow[];
  draft: ThermalDraft;
  onDraft: (next: ThermalDraft) => void;
}) {
  const t = useT().systems.thermalNet;
  const optics = (row: ThermalCoatingRow, field: Optics) => {
    const v = row[field];
    if (!v) return <span className="text-ink-4">{t.none}</span>;
    return (
      <EditableQuantity
        display={v.display}
        initial={quantityInputText(v)}
        dimension={v.dimension}
        label={`${row.id} ${field === "absorptivity" ? t.absorptivity : t.emissivity}`}
        onCommit={(text) => onDraft(withCell(draft, coatingCell(row.id, field, text)))}
      />
    );
  };
  return (
    <TablePanel title={t.coatingsTitle} caption={t.coatingsCaption}>
      {coatings.length === 0 ? (
        <Empty>{t.noCoatings}</Empty>
      ) : (
        <DataTable
          head={
            <>
              <Th>{t.id}</Th>
              <Th>{t.label}</Th>
              <Th>{t.absorptivity}</Th>
              <Th>{t.emissivity}</Th>
              <Th>{t.usedBy}</Th>
              <Th>
                <span className="sr-only">{t.removeCoating("")}</span>
              </Th>
            </>
          }
        >
          {coatings.map((row) => (
            <tr key={row.id} className="hover:bg-panel-2/60">
              <Td mono>{row.id}</Td>
              <Td>
                <CommitInput
                  label={`${row.id} ${t.label}`}
                  value={row.label ?? ""}
                  className="max-w-56"
                  onCommit={(text) => onDraft(withCell(draft, coatingCell(row.id, "label", text)))}
                />
              </Td>
              <Td>{optics(row, "absorptivity")}</Td>
              <Td>{optics(row, "emissivity")}</Td>
              <Td mono className="text-ink-2">
                {row.used_by.length > 0 ? row.used_by.join(", ") : t.none}
              </Td>
              <Td className="text-right">
                <Button
                  variant="ghost"
                  size="icon-xs"
                  aria-label={t.removeCoating(row.id)}
                  title={t.removeCoating(row.id)}
                  onClick={() => onDraft(withCell(draft, { cell: "coating_remove", id: row.id }))}
                >
                  <Trash2 />
                </Button>
              </Td>
            </tr>
          ))}
        </DataTable>
      )}
      <AddCoating draft={draft} onDraft={onDraft} />
    </TablePanel>
  );
}
