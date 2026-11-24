import { useId, useState } from "react";
import { useEnumOptions } from "@/api/labels";
import type { Distribution, EvidenceSource, SourceKind } from "@/api/types";
import { type Dict, useT } from "@/i18n";
import { Field } from "@/kit/forms/Field";
import { NativeSelect } from "@/kit/forms/NativeSelect";
import { TextInput } from "@/kit/forms/TextInput";
import { OpDialog } from "@/ops/OpDialog";
import { draftOf, type SourceDraft, setSourceOp, type TextField } from "./sources";

/** 欄の並び（ファイルの書き順と同じ）と、等幅で出すか・補足 */
const LAYOUT: { field: TextField; mono?: boolean; hint?: (m: Dict["sources"]) => string }[] = [
  { field: "label" },
  { field: "ref", mono: true, hint: (m) => m.refHint },
  { field: "revision" },
  { field: "retrieved", mono: true, hint: (m) => m.dateHint },
  { field: "sha256", mono: true, hint: (m) => m.sha256Hint },
  { field: "date", mono: true, hint: (m) => m.dateHint },
  { field: "by" },
  { field: "note" },
];

/**
 * 出典を書く（`set_source`。`fune source set` と同じ Op）。`source` が null なら新しい出典で、ID の欄を出す。
 * 日付と SHA-256 の形・ID の規則はサーバーの dry_run が検査し、欄の下に出す
 */
export function SourceDialog({
  source,
  open,
  onOpenChange,
}: {
  source: EvidenceSource | null;
  open: boolean;
  onOpenChange: (open: boolean) => void;
}) {
  const m = useT().sources;
  const id = useId();
  const kinds = useEnumOptions("source_kind");
  const distributions = useEnumOptions("distribution");
  const [draft, setDraft] = useState<SourceDraft>(() => draftOf(source));
  const ops = (() => {
    const op = setSourceOp(draft);
    return op ? [op] : null;
  })();
  const set = (patch: Partial<SourceDraft>) => setDraft((d) => ({ ...d, ...patch }));
  return (
    <OpDialog
      open={open}
      onOpenChange={onOpenChange}
      wide
      title={source ? m.editTitle(source.id) : m.addTitle}
      description={m.dialogDescription}
      ops={ops}
      summary={m.summarySet(draft.id.trim())}
    >
      <div className="grid grid-cols-2 gap-3">
        {!source && (
          <Field label={m.id} htmlFor={`${id}-id`}>
            <TextInput
              id={`${id}-id`}
              mono
              value={draft.id}
              onChange={(e) => set({ id: e.target.value })}
            />
          </Field>
        )}
        <Field label={m.kind} htmlFor={`${id}-kind`}>
          <NativeSelect
            id={`${id}-kind`}
            value={draft.kind}
            options={kinds}
            onChange={(e) => set({ kind: e.target.value as SourceKind })}
          />
        </Field>
        <Field label={m.distribution} htmlFor={`${id}-distribution`}>
          <NativeSelect
            id={`${id}-distribution`}
            value={draft.distribution}
            placeholder={m.none}
            options={distributions}
            onChange={(e) => set({ distribution: e.target.value as Distribution | "" })}
          />
        </Field>
        {LAYOUT.map(({ field, mono, hint }) => (
          <Field key={field} label={m[field]} htmlFor={`${id}-${field}`} hint={hint?.(m)}>
            <TextInput
              id={`${id}-${field}`}
              mono={mono}
              value={draft[field]}
              onChange={(e) => set({ [field]: e.target.value })}
            />
          </Field>
        ))}
      </div>
    </OpDialog>
  );
}
