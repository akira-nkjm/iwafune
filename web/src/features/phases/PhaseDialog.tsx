import { useState } from "react";
import { useCommandOps } from "@/api/ops";
import { useDesign } from "@/api/queries";
import type { PhaseText } from "@/api/types";
import { useT } from "@/i18n";
import { Field } from "@/kit/forms/Field";
import { NativeSelect } from "@/kit/forms/NativeSelect";
import { TextInput } from "@/kit/forms/TextInput";
import { idWithName } from "@/lib/design";
import { OpDialog } from "@/ops/OpDialog";
import { useOpCheck } from "@/ops/use-op-check";
import { phaseCommand } from "./phase-form";

type TextKey = Exclude<keyof PhaseText, "id" | "configuration">;

/**
 * フェーズの追加・編集（`fune phase add|set` と同じ引数）。打った文字列のまま service に送り、Op にしてから確定する。
 * 編集は欄の初期値に API の `PhaseView.text` を使う（軌道と姿勢則の上書きは `SetPhase` が今のまま残す）
 */
export function PhaseDialog({
  initial,
  adding,
  onOpenChange,
}: {
  initial: PhaseText;
  adding: boolean;
  onOpenChange: (open: boolean) => void;
}) {
  const t = useT().phases;
  const td = t.dialog;
  const [text, setText] = useState<PhaseText>(initial);
  const configurations = useDesign().data?.configurations ?? [];
  const made = useCommandOps(phaseCommand(text, adding));
  const failed = useOpCheck(made.ops).failed;
  const error = made.error;
  const set = (key: keyof PhaseText, value: string) => setText((s) => ({ ...s, [key]: value }));
  const input = (key: TextKey, label: string, placeholder?: string) => (
    <Field label={label} htmlFor={`ph-${key}`}>
      <TextInput
        id={`ph-${key}`}
        mono={key === "order" || key === "duration" || key === "modes"}
        value={text[key] ?? ""}
        placeholder={placeholder}
        onChange={(e) => set(key, e.target.value)}
      />
    </Field>
  );
  return (
    <OpDialog
      open
      wide
      onOpenChange={onOpenChange}
      title={adding ? td.addTitle : td.editTitle(text.id)}
      description={td.description}
      ops={made.ops}
      summary={adding ? td.summaryAdd(text.id) : td.summaryEdit(text.id)}
      confirmLabel={adding ? td.confirmAdd : td.confirmEdit}
    >
      <div className="grid grid-cols-2 gap-3">
        <Field label={t.id} htmlFor="ph-id" error={error}>
          <TextInput
            id="ph-id"
            mono
            value={text.id}
            disabled={!adding}
            invalid={failed || !!error}
            placeholder="LEOP"
            onChange={(e) => set("id", e.target.value)}
          />
        </Field>
        {input("label", td.label)}
        {input("order", td.order, "1")}
        {input("duration", td.duration, "30 d")}
        {input("start", td.start)}
        {input("end", td.end)}
        {input("modes", td.modes, "SAFE,NOMINAL")}
        <Field label={td.configuration} htmlFor="ph-configuration">
          <NativeSelect
            id="ph-configuration"
            className="w-full"
            value={text.configuration ?? ""}
            placeholder={td.configurationNone}
            options={configurations.map((c) => ({
              value: c.id,
              label: idWithName(c),
            }))}
            onChange={(e) => set("configuration", e.target.value)}
          />
        </Field>
      </div>
    </OpDialog>
  );
}
