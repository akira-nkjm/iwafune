import type { Provenance } from "@/api/types";
import { useT } from "@/i18n";
import { Field } from "@/kit/Field";
import { TextInput } from "@/kit/TextInput";

/** 出典とメモ（任意）。どちらも空なら null */
export function ProvenanceFields({
  value,
  onChange,
}: {
  value: { source: string; note: string };
  onChange: (v: { source: string; note: string }) => void;
}) {
  const tp = useT().components.param;
  return (
    <div className="grid grid-cols-2 gap-3">
      <Field label={tp.source} htmlFor="param-source">
        <TextInput
          id="param-source"
          mono
          value={value.source}
          placeholder="em_test_2026_10"
          onChange={(e) => onChange({ ...value, source: e.target.value })}
        />
      </Field>
      <Field label={tp.note} htmlFor="param-note">
        <TextInput
          id="param-note"
          value={value.note}
          placeholder={tp.notePlaceholder}
          onChange={(e) => onChange({ ...value, note: e.target.value })}
        />
      </Field>
    </div>
  );
}

/** 入力から Op の出典にする */
export function provenanceOf(v: { source: string; note: string }): Provenance | null {
  return v.source || v.note ? { source: v.source || null, note: v.note || null } : null;
}
