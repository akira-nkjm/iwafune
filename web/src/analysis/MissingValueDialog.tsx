import { useCallback, useState } from "react";
import { useTextOp } from "@/api/ops";
import { useT } from "@/i18n";
import { Field } from "@/kit/forms/Field";
import { TextInput } from "@/kit/forms/TextInput";
import { Segmented } from "@/kit/selection/Segmented";
import { OpDialog } from "@/ops/OpDialog";
import { QuantityInput } from "@/quantity/QuantityInput";
import { type ValueFix, type ValueScope, valueSetText } from "./missing-fix";

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
  const [scope, setScope] = useState<ValueScope>("part");
  const [value, setValue] = useState("");
  const onChange = useCallback((text: string) => setValue(text), []);
  const made = useTextOp(valueSetText(fix, scope, value));
  const path = scope === "catalog" && fix.catalog ? fix.catalog.path : fix.path;
  return (
    <OpDialog
      open
      wide
      onOpenChange={onOpenChange}
      title={t.title(fix.path)}
      description={t.description}
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
        hint={fix.dimension ? t.dimension(fix.dimension) : undefined}
      >
        {fix.dimension ? (
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
