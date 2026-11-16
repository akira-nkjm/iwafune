import { useT } from "@/i18n";
import { Field } from "@/kit/forms/Field";
import { TextInput } from "@/kit/forms/TextInput";

/** 打ったままの出典・資料の中の位置・メモ（空なら付けない） */
export interface Provenance {
  source: string;
  at: string;
  note: string;
}

/** 出典と資料の中の位置とメモ（任意）。打ったまま `SetText` の source・at・note に渡す */
export function ProvenanceFields({
  value,
  onChange,
}: {
  value: Provenance;
  onChange: (v: Provenance) => void;
}) {
  const tp = useT().components.param;
  return (
    <div className="grid grid-cols-3 gap-3">
      <Field label={tp.source} htmlFor="param-source">
        <TextInput
          id="param-source"
          mono
          value={value.source}
          placeholder="em_test_2026_10"
          onChange={(e) => onChange({ ...value, source: e.target.value })}
        />
      </Field>
      <Field label={tp.at} htmlFor="param-at">
        <TextInput
          id="param-at"
          value={value.at}
          placeholder={tp.atPlaceholder}
          onChange={(e) => onChange({ ...value, at: e.target.value })}
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
