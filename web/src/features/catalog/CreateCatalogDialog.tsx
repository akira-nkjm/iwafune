import { useState } from "react";
import type { DesignView, Maturity, Op } from "@/api/types";
import { localized, useT } from "@/i18n";
import { Field } from "@/kit/Field";
import { NativeSelect } from "@/kit/NativeSelect";
import { TextInput } from "@/kit/TextInput";
import { OpDialog } from "@/ops/OpDialog";
import { useOpCheck } from "@/ops/use-op-check";

/** カタログ品を作る（create_catalog_item）。部品から作ると、その部品の値を写す（copy_from_part） */
export function CreateCatalogDialog({
  design,
  open,
  onOpenChange,
  onCreated,
}: {
  design: DesignView;
  open: boolean;
  onOpenChange: (o: boolean) => void;
  onCreated: (id: string) => void;
}) {
  const t = useT();
  const [id, setId] = useState("local/");
  const [kind, setKind] = useState("");
  const [label, setLabel] = useState("");
  const [labelEn, setLabelEn] = useState("");
  const [maturity, setMaturity] = useState("");
  const [fromPart, setFromPart] = useState("");

  const kindOf = fromPart ? (design.parts.find((p) => p.id === fromPart)?.kind ?? "") : kind;
  const ops: Op[] | null =
    id !== "local/" && kindOf
      ? [
          {
            op: "create_catalog_item",
            id,
            kind: kindOf,
            label: label || null,
            label_en: labelEn || null,
            maturity: (maturity || null) as Maturity | null,
            copy_from_part: fromPart || null,
          },
        ]
      : null;
  // ID の規則と重複はサーバー（dry_run）が検査する
  const error = useOpCheck(ops).error;
  return (
    <OpDialog
      open={open}
      onOpenChange={onOpenChange}
      title={t.catalog.add}
      description={t.catalog.createDesc}
      ops={ops}
      summary={t.catalog.summaryCreate(id)}
      confirmLabel={t.common.add}
      onDone={() => onCreated(id)}
    >
      <Field label={t.catalog.id} htmlFor="cat-id" error={error} hint={t.catalog.idLocal}>
        <TextInput
          id="cat-id"
          mono
          value={id}
          invalid={!!error}
          onChange={(e) => setId(e.target.value)}
        />
      </Field>
      <Field label={t.catalog.fromPart} htmlFor="cat-from">
        <NativeSelect
          id="cat-from"
          value={fromPart}
          placeholder={t.catalog.fromPartNone}
          onChange={(e) => setFromPart(e.target.value)}
          options={design.parts.map((p) => ({ value: p.id, label: `${p.id}（${p.kind}）` }))}
        />
      </Field>
      {!fromPart && (
        <Field label={t.catalog.kind} htmlFor="cat-kind">
          <NativeSelect
            id="cat-kind"
            value={kind}
            placeholder="—"
            onChange={(e) => setKind(e.target.value)}
            options={design.defs.kinds.map((k) => ({
              value: k.id,
              label: `${k.id}（${localized(k.label, k.label_en)}）`,
            }))}
          />
        </Field>
      )}
      <div className="grid grid-cols-2 gap-3">
        <Field label={t.catalog.label} htmlFor="cat-label">
          <TextInput id="cat-label" value={label} onChange={(e) => setLabel(e.target.value)} />
        </Field>
        <Field label={t.catalog.labelEn} htmlFor="cat-label-en">
          <TextInput
            id="cat-label-en"
            value={labelEn}
            onChange={(e) => setLabelEn(e.target.value)}
          />
        </Field>
      </div>
      <Field label={t.catalog.maturity} htmlFor="cat-maturity">
        <NativeSelect
          id="cat-maturity"
          value={maturity}
          placeholder="—"
          onChange={(e) => setMaturity(e.target.value)}
          options={["off_the_shelf", "modified", "new"].map((m) => ({ value: m, label: m }))}
        />
      </Field>
    </OpDialog>
  );
}
