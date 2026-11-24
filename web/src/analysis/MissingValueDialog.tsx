import { useCallback, useState } from "react";
import { useTextOp } from "@/api/ops";
import { useT } from "@/i18n";
import { Field } from "@/kit/forms/Field";
import { NativeSelect } from "@/kit/forms/NativeSelect";
import { TextInput } from "@/kit/forms/TextInput";
import { Segmented } from "@/kit/selection/Segmented";
import { OpDialog } from "@/ops/OpDialog";
import { QuantityInput } from "@/quantity/QuantityInput";
import { useDimensionLabel } from "@/quantity/use-dimension-label";
import { choiceLabel } from "./MissingPickDialog";
import { type ValueFix, type ValueScope, valueSetText } from "./missing-fix";

/** 入力欄の補足（次元と入力の例。例は service が次元から作る） */
function hintOf(
  dimension: string | null,
  example: string | null,
  t: { dimension: (d: string) => string; example: (e: string) => string },
  dimensionLabel: (name: string) => string,
): string | undefined {
  const parts = [
    dimension ? t.dimension(dimensionLabel(dimension)) : null,
    example ? t.example(example) : null,
  ];
  const text = parts.filter((x) => x !== null).join(" · ");
  return text || undefined;
}

/**
 * 足りない値をその場で入れるダイアログ（部品の属性・フィールド・設定）。打った文字列は `POST /ops/from_text`（`fune set`・
 * `fune catalog set` と同じ service）で Op にし、確定前に CLI と変わるファイルを見せる。次元が分かれば単位付きの入力欄にする。
 * 部品がカタログ品を選んでいれば、部品の上書きか品の値かを選べる。確定後の再計算は編集の後の無効化に任せる。
 */
export function MissingValueDialog({
  fix,
  onOpenChange,
}: {
  fix: ValueFix;
  onOpenChange: (open: boolean) => void;
}) {
  const t = useT().fixes.value;
  const dimensionLabel = useDimensionLabel();
  const [scope, setScope] = useState<ValueScope>("part");
  const [value, setValue] = useState("");
  const onChange = useCallback((text: string) => setValue(text), []);
  const made = useTextOp(valueSetText(fix, scope, value));
  // 単位付きの入力欄（QuantityInput）は次元を自分で出すので、補足では次元を重ねない（例だけ出す）
  const quantityInput = fix.choices.length === 0 && fix.dimension !== null;
  const path = scope === "catalog" && fix.catalog ? fix.catalog.path : fix.path;
  return (
    <OpDialog
      open
      wide
      onOpenChange={onOpenChange}
      title={t.title(fix.path)}
      description={fix.choices.length > 0 ? t.descriptionChoice : t.description}
      ops={made.op ? [made.op] : null}
      summary={t.summary(path)}
      confirmLabel={t.confirm}
    >
      {fix.catalog && (
        <Field label={t.scope} hint={t.scopeHint}>
          <Segmented
            label={t.scope}
            value={scope}
            onChange={setScope}
            options={[
              { value: "part", label: t.part },
              { value: "catalog", label: t.catalog(fix.catalog.item) },
            ]}
          />
        </Field>
      )}
      <Field
        label={t.label}
        htmlFor="missing-value"
        error={made.error}
        hint={hintOf(quantityInput ? null : fix.dimension, fix.example, t, dimensionLabel)}
      >
        {fix.choices.length > 0 ? (
          <NativeSelect
            id="missing-value"
            value={value}
            placeholder={t.choose}
            options={fix.choices.map((c) => ({ value: c.value, label: choiceLabel(c) }))}
            onChange={(e) => onChange(e.target.value)}
          />
        ) : fix.dimension ? (
          <QuantityInput
            initial=""
            dimension={fix.dimension}
            label={t.label}
            autoFocus
            onChangeValid={onChange}
          />
        ) : (
          <TextInput
            id="missing-value"
            mono
            autoFocus
            value={value}
            onChange={(e) => onChange(e.target.value)}
          />
        )}
      </Field>
    </OpDialog>
  );
}
