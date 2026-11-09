import { useState } from "react";
import { useEnumOptions } from "@/api/labels";
import type { DesignView, Maturity, Op } from "@/api/types";
import { useT } from "@/i18n";
import { Field } from "@/kit/Field";
import { NativeSelect } from "@/kit/NativeSelect";
import { TextInput } from "@/kit/TextInput";
import { defWithId } from "@/lib/design";
import { OpDialog } from "@/ops/OpDialog";
import { useOpCheck } from "@/ops/use-op-check";
import { catalogItemId, writableCatalogs } from "./catalog";

/**
 * カタログ品を作る（create_catalog_item）。置き先は書き込めるカタログ（API の `ProjectInfo.catalogs`）から選ぶ。
 * 部品から作ると、その部品の値を写す（copy_from_part）
 */
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
  const maturities = useEnumOptions("maturity");
  const writable = writableCatalogs(design.project);
  const [scope, setScope] = useState(writable[0] ?? "");
  const [name, setName] = useState("");
  const [kind, setKind] = useState("");
  const [label, setLabel] = useState("");
  const [labelEn, setLabelEn] = useState("");
  const [maturity, setMaturity] = useState("");
  const [fromPart, setFromPart] = useState("");

  const id = scope && name ? catalogItemId(scope, name) : "";
  const kindOf = fromPart ? (design.parts.find((p) => p.id === fromPart)?.kind ?? "") : kind;
  const ops: Op[] | null =
    id && kindOf
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
      {writable.length === 0 ? (
        <p className="rounded-control bg-warn-bg px-3 py-2 text-warn-ink text-xs">
          {t.catalog.noWritable}
        </p>
      ) : (
        <div className="grid grid-cols-[140px_1fr] gap-3">
          <Field label={t.catalog.scope} htmlFor="cat-scope">
            <NativeSelect
              id="cat-scope"
              value={scope}
              disabled={writable.length === 1}
              onChange={(e) => setScope(e.target.value)}
              options={writable.map((c) => ({ value: c, label: c }))}
            />
          </Field>
          <Field
            label={t.catalog.itemName}
            htmlFor="cat-name"
            error={error}
            hint={t.catalog.itemNameHint}
          >
            <TextInput
              id="cat-name"
              mono
              value={name}
              invalid={!!error}
              onChange={(e) => setName(e.target.value)}
            />
          </Field>
        </div>
      )}
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
            options={design.defs.kinds.map((k) => ({ value: k.id, label: defWithId(k) }))}
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
          options={maturities}
        />
      </Field>
    </OpDialog>
  );
}
