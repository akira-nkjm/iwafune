import { useCallback, useState } from "react";
import { useTextOp } from "@/api/ops";
import { useT } from "@/i18n";
import { Field } from "@/kit/forms/Field";
import { NativeSelect } from "@/kit/forms/NativeSelect";
import { TextInput } from "@/kit/forms/TextInput";
import { OpDialog } from "@/ops/OpDialog";
import { QuantityInput } from "@/quantity/QuantityInput";
import { type PartFix, partSetText } from "./missing-fix";

/**
 * 役割のどれかの部品を選んで書くダイアログ。値を入れる候補（その役割の部品はあるが値が無い）は部品を選んで値を打ち、
 * 割り当てる候補（その役割の部品が無い）は既にある部品と、その役割の種別を選ぶ。どちらも `fune set` と同じ文字列を
 * service で Op にし、確定前に CLI と変わるファイルを見せる。選べる部品・種別は API が絞って返す。
 */
export function MissingPartDialog({
  fix,
  onOpenChange,
}: {
  fix: PartFix;
  onOpenChange: (open: boolean) => void;
}) {
  const t = useT().fixes.part;
  const assign = fix.kind === "assign_part";
  const [part, setPart] = useState("");
  const [value, setValue] = useState(assign && fix.kinds.length === 1 ? (fix.kinds[0] ?? "") : "");
  const onChange = useCallback((text: string) => setValue(text), []);
  const made = useTextOp(partSetText(fix, part, value));
  // 単位付きの入力欄（QuantityInput）は id を自分で持ち、aria-label で名前を持つので、ラベルの `for` は付けない（ほかのダイアログと同じ）
  const quantityInput = !assign && fix.choices.length === 0 && fix.dimension !== null;
  const choice = (v: string) => ({ value: v, label: v });
  return (
    <OpDialog
      open
      wide
      onOpenChange={onOpenChange}
      title={assign ? t.assignTitle : t.setTitle(fix.attr)}
      description={assign ? t.assignDescription : t.setDescription}
      ops={made.op ? [made.op] : null}
      summary={assign ? t.assignSummary(part, value) : t.setSummary(part, fix.attr)}
      confirmLabel={t.confirm}
    >
      <Field label={t.part} htmlFor="missing-part" error={made.error}>
        <NativeSelect
          id="missing-part"
          value={part}
          placeholder={t.choose}
          options={fix.parts.map(choice)}
          onChange={(e) => setPart(e.target.value)}
        />
      </Field>
      {assign && fix.kinds.length > 1 && (
        <Field label={t.kind} htmlFor="missing-part-kind">
          <NativeSelect
            id="missing-part-kind"
            value={value}
            placeholder={t.choose}
            options={fix.kinds.map(choice)}
            onChange={(e) => onChange(e.target.value)}
          />
        </Field>
      )}
      {!assign && (
        <Field
          label={t.value}
          htmlFor={quantityInput ? undefined : "missing-part-value"}
          hint={fix.example ? t.example(fix.example) : undefined}
        >
          {fix.choices.length > 0 ? (
            <NativeSelect
              id="missing-part-value"
              value={value}
              placeholder={t.choose}
              options={fix.choices.map((c) => ({ value: c.value, label: c.label }))}
              onChange={(e) => onChange(e.target.value)}
            />
          ) : fix.dimension ? (
            <QuantityInput
              initial=""
              dimension={fix.dimension}
              label={t.value}
              onChangeValid={onChange}
            />
          ) : (
            <TextInput
              id="missing-part-value"
              mono
              value={value}
              onChange={(e) => onChange(e.target.value)}
            />
          )}
        </Field>
      )}
    </OpDialog>
  );
}
