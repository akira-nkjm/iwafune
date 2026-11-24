import { Plus } from "lucide-react";
import { useCallback, useState } from "react";
import type { ThermalCouplingKind, ThermalCouplingsTable } from "@/api/types";
import { localized, useT } from "@/i18n";
import { NativeSelect } from "@/kit/forms/NativeSelect";
import { QuantityInput } from "@/quantity/QuantityInput";
import { Button } from "@/ui/button";
import { type ThermalDraft, withCell } from "./draft";

/**
 * 結合を足す欄: 種類・両端の節点（API の一覧から選ぶ）・値。値の単位は種類の次元で検査し、足すと下書きに入る
 * （同じ対が逆向きで既にある・同じ節点どうしなどの誤りは、保存の帯の検査が理由を出す）。
 */
export function AddCoupling({
  table,
  draft,
  onDraft,
}: {
  table: ThermalCouplingsTable;
  draft: ThermalDraft;
  onDraft: (next: ThermalDraft) => void;
}) {
  const t = useT().systems.thermalNet;
  const [kind, setKind] = useState<ThermalCouplingKind>(table.kinds[0]?.kind ?? "conductive");
  const [a, setA] = useState("");
  const [b, setB] = useState("");
  const [value, setValue] = useState<{ text: string; valid: boolean }>({ text: "", valid: false });
  // 足したあとは値の欄を作り直して空にする
  const [round, setRound] = useState(0);
  const current = table.kinds.find((k) => k.kind === kind);
  const onValid = useCallback((text: string, valid: boolean) => setValue({ text, valid }), []);
  const nodes = table.nodes.map((n) => ({ value: n, label: n }));
  const ready = a !== "" && b !== "" && value.valid;
  const add = () => {
    onDraft(withCell(draft, { cell: "coupling", kind, a, b, value: value.text }));
    setA("");
    setB("");
    setRound((n) => n + 1);
    setValue({ text: "", valid: false });
  };
  return (
    <fieldset className="flex flex-wrap items-start gap-2 border-line border-t p-3">
      <legend className="sr-only">{t.addTitle}</legend>
      <span className="self-center font-medium text-ink-2 text-xs">{t.addTitle}</span>
      <NativeSelect
        size="sm"
        aria-label={t.kind}
        value={kind}
        options={table.kinds.map((k) => ({
          value: k.kind,
          label: localized(k.label.label, k.label.label_en),
        }))}
        onChange={(e) => {
          const picked = table.kinds.find((k) => k.kind === e.target.value);
          if (picked) setKind(picked.kind);
        }}
      />
      <NativeSelect
        size="sm"
        aria-label={t.nodeA}
        value={a}
        placeholder={t.addPick}
        options={nodes}
        onChange={(e) => setA(e.target.value)}
      />
      <NativeSelect
        size="sm"
        aria-label={t.nodeB}
        value={b}
        placeholder={t.addPick}
        options={nodes}
        onChange={(e) => setB(e.target.value)}
      />
      <QuantityInput
        key={`${kind}:${round}`}
        compact
        initial=""
        dimension={current?.dimension ?? null}
        label={t.valueFor(current ? localized(current.label.label, current.label.label_en) : kind)}
        className="w-36"
        onChangeValid={onValid}
      />
      <Button size="sm" variant="outline" disabled={!ready} onClick={add}>
        <Plus />
        {t.add}
      </Button>
    </fieldset>
  );
}
