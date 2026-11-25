import { Plus, Trash2 } from "lucide-react";
import { Field } from "@/kit/forms/Field";
import { TextInput } from "@/kit/forms/TextInput";
import { Button } from "@/ui/button";

/**
 * 日付つきの遷移の途中のスイングバイ（`--flyby` と同じ `天体@日付[,min_altitude=400 km][,powered=false]` を 1 行に 1 回）。
 * 打った文字列のまま渡し、読むのは service（`ManeuverFlyby::from_text`）。書かれた行は編集しなくても欄の値としてそのまま残る
 */
export function FlybysField({
  label,
  hint,
  addLabel,
  removeLabel,
  value,
  onChange,
}: {
  label: string;
  hint: string;
  addLabel: string;
  removeLabel: string;
  value: string[];
  onChange: (value: string[]) => void;
}) {
  return (
    <Field label={label} hint={hint} className="col-span-2">
      <div className="grid gap-1.5">
        {value.map((line, i) => (
          // biome-ignore lint/suspicious/noArrayIndexKey: 行の並びが識別子（編集中の文字列は一意でない）
          <div key={i} className="flex gap-1.5">
            <TextInput
              mono
              value={line}
              placeholder="venus@2029-02-01T00:00:00Z"
              aria-label={`${label} ${i + 1}`}
              onChange={(e) => onChange(value.map((v, j) => (j === i ? e.target.value : v)))}
            />
            <Button
              size="sm"
              variant="ghost"
              aria-label={removeLabel}
              onClick={() => onChange(value.filter((_, j) => j !== i))}
            >
              <Trash2 size={13} />
            </Button>
          </div>
        ))}
        <div>
          <Button size="sm" variant="ghost" onClick={() => onChange([...value, ""])}>
            <Plus size={13} /> {addLabel}
          </Button>
        </div>
      </div>
    </Field>
  );
}
