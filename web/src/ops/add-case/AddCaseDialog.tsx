import { useState } from "react";
import { useEnumOptions } from "@/api/labels";
import { useCommandOps } from "@/api/ops";
import type { DesignView } from "@/api/types";
import { useT } from "@/i18n";
import { Field } from "@/kit/forms/Field";
import { NativeSelect } from "@/kit/forms/NativeSelect";
import { TextInput } from "@/kit/forms/TextInput";
import { OpDialog } from "@/ops/OpDialog";
import { useOpCheck } from "@/ops/use-op-check";
import { caseChoices, caseSetCommand, emptyCase, type NewCase } from "./case-form";

/**
 * 解析ケースの追加（SetCase。`fune case set` と同じ引数）。解析ケースのページのボタンと、足りない入力の行（`AddListItem`）から開く。
 * モード・フェーズ・熱の環境条件・不確かさの側は API が返した選択肢から選び、量・周回数・時刻は打った文字列のまま送る（読むのは service）。
 * 同じ ID のケースがあれば置き換わり、書かなかった欄は消える（プレビューが消える欄を知らせる）。
 * `initialId` は足りない入力が名指ししたケースの ID
 */
export function AddCaseDialog({
  design,
  initialId,
  onOpenChange,
}: {
  design: DesignView;
  initialId?: string | null;
  onOpenChange: (open: boolean) => void;
}) {
  const tc = useT().conditions.addCase;
  const [fresh, setFresh] = useState<NewCase>(emptyCase(initialId));
  const sides = useEnumOptions("uncertainty_side");
  const choices = caseChoices(design);
  const made = useCommandOps(caseSetCommand(fresh));
  const failed = useOpCheck(made.ops).failed;
  const error = made.error;
  const set = (key: keyof NewCase) => (e: { target: { value: string } }) =>
    setFresh((f) => ({ ...f, [key]: e.target.value }));
  const text = (key: keyof NewCase, label: string, placeholder?: string, hint?: string) => (
    <Field label={label} htmlFor={`case-${key}`} hint={hint}>
      <TextInput
        id={`case-${key}`}
        mono
        value={fresh[key]}
        placeholder={placeholder}
        onChange={set(key)}
      />
    </Field>
  );
  const select = (
    key: keyof NewCase,
    label: string,
    options: { value: string; label: string }[],
  ) => (
    <Field label={label} htmlFor={`case-${key}`}>
      <NativeSelect
        id={`case-${key}`}
        value={fresh[key]}
        placeholder={tc.none}
        options={options}
        onChange={set(key)}
      />
    </Field>
  );
  return (
    <OpDialog
      open
      onOpenChange={onOpenChange}
      title={tc.title}
      description={tc.description}
      ops={made.ops}
      summary={tc.summary(fresh.id.trim())}
      confirmLabel={tc.confirm}
      wide
    >
      <div className="grid grid-cols-2 gap-3">
        <Field label="ID" htmlFor="case-id" error={error}>
          <TextInput
            id="case-id"
            mono
            value={fresh.id}
            invalid={failed || !!error}
            placeholder="hot_imaging"
            onChange={set("id")}
          />
        </Field>
        <Field label={tc.label} htmlFor="case-label">
          <TextInput id="case-label" value={fresh.label} onChange={set("label")} />
        </Field>
        {select("mode", tc.mode, choices.modes)}
        {select("phase", tc.phase, choices.phases)}
        {select("side", tc.side, sides)}
        {select("thermal_case", tc.thermalCase, choices.thermalCases)}
        {text("beta", tc.beta, "60 deg", tc.quantityHint)}
        {text("orbits", tc.orbits, "5")}
        {text("sun_distance", tc.sunDistance, "0.7 AU")}
        {text("epoch", tc.epoch, "2031-05-02T00:00:00Z", tc.epochHint)}
      </div>
      <Field label={tc.descriptionLabel} htmlFor="case-description">
        <TextInput id="case-description" value={fresh.description} onChange={set("description")} />
      </Field>
    </OpDialog>
  );
}
