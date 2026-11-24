import { useQuery } from "@tanstack/react-query";
import { useState } from "react";
import { useEnumOptions } from "@/api/labels";
import { useCommandOps } from "@/api/ops";
import { q } from "@/api/queries";
import type { DesignView, RequirementText } from "@/api/types";
import { localized, useT } from "@/i18n";
import { Field } from "@/kit/forms/Field";
import { NativeSelect } from "@/kit/forms/NativeSelect";
import { TextInput } from "@/kit/forms/TextInput";
import { OpDialog } from "@/ops/OpDialog";
import { useOpCheck } from "@/ops/use-op-check";
import { requirementCommand } from "./requirement-form";

type TextKey = Exclude<keyof RequirementText, "id" | "kind" | "statement" | "quantity">;

/**
 * 要求の追加・編集（`fune requirement add|set` と同じ引数）。打った文字列のまま service に送り、Op にしてから確定する。
 * 編集は欄の初期値に API の `RequirementView.text` を使い、丸ごと書き換える（`SetRequirement`）。
 * 制約の次元・参照の実在・確かめる先の組は、dry_run の検査（`fune check` と同じ規則）が欄の下に出す
 */
export function RequirementDialog({
  design,
  initial,
  adding,
  onOpenChange,
}: {
  design: DesignView;
  initial: RequirementText;
  adding: boolean;
  onOpenChange: (open: boolean) => void;
}) {
  const t = useT().requirements;
  const td = t.dialog;
  const [text, setText] = useState<RequirementText>(initial);
  const kinds = useEnumOptions("constraint_kind");
  const budgetOptions = useEnumOptions("budget_kind");
  const analyses = useQuery(q.analyses()).data ?? [];
  const dimensions = useQuery(q.units()).data?.dimensions ?? [];
  const made = useCommandOps(requirementCommand(text, adding));
  const failed = useOpCheck(made.ops).failed;
  const error = made.error;
  const set = (key: keyof RequirementText, value: string) =>
    setText((s) => ({ ...s, [key]: value }));
  const input = (key: TextKey, label: string, placeholder?: string, hint?: string) => (
    <Field label={label} htmlFor={`rq-${key}`} hint={hint}>
      <TextInput
        id={`rq-${key}`}
        mono
        value={text[key] ?? ""}
        placeholder={placeholder}
        onChange={(e) => set(key, e.target.value)}
      />
    </Field>
  );
  const select = (key: TextKey, label: string, options: { value: string; label: string }[]) => (
    <Field label={label} htmlFor={`rq-${key}`}>
      <NativeSelect
        id={`rq-${key}`}
        value={text[key] ?? ""}
        placeholder={td.unset}
        options={options}
        onChange={(e) => set(key, e.target.value)}
      />
    </Field>
  );
  const range = text.kind === "range";
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
        <Field label={t.id} htmlFor="rq-id" error={error}>
          <TextInput
            id="rq-id"
            mono
            value={text.id}
            disabled={!adding}
            invalid={failed || !!error}
            placeholder="R_MASS"
            onChange={(e) => set("id", e.target.value)}
          />
        </Field>
        {input("label", td.label)}
        <Field label={td.statementJa} htmlFor="rq-statement" className="col-span-2">
          <TextInput
            id="rq-statement"
            value={text.statement}
            onChange={(e) => set("statement", e.target.value)}
          />
        </Field>
        {input("statement_en", td.statementEn)}
        {select(
          "source",
          td.source,
          design.sources.map((s) => ({ value: s.id, label: s.id })),
        )}
        <Field label={td.quantity} htmlFor="rq-quantity" hint={td.quantityHint}>
          <NativeSelect
            id="rq-quantity"
            value={text.quantity}
            placeholder={td.unset}
            options={dimensions.map((d) => ({
              value: d.name,
              label: localized(d.label, d.label_en),
            }))}
            onChange={(e) => set("quantity", e.target.value)}
          />
        </Field>
        <Field label={td.kind} htmlFor="rq-kind">
          <NativeSelect
            id="rq-kind"
            value={text.kind}
            options={kinds}
            onChange={(e) => set("kind", e.target.value)}
          />
        </Field>
        {range ? (
          <>
            {input("min", td.min, "0 degC")}
            {input("max", td.max, "40 degC")}
          </>
        ) : (
          <>
            {input("value", td.value, "12 kg")}
            {text.kind === "equal" && input("tolerance", td.tolerance, "0.5 K")}
          </>
        )}
        {select("all_cases", td.allCases, [
          { value: "true", label: td.yes },
          { value: "false", label: td.no },
        ])}
        {input("cases", td.cases, "hot_imaging,cold_safe", td.casesHint)}
        {select(
          "analysis",
          td.analysis,
          analyses.map((a) => ({ value: a.info.id, label: a.info.id })),
        )}
        {input("check", td.check, "dod", td.checkHint)}
        {select("budget", td.budget, budgetOptions)}
        <Field label={td.input} htmlFor="rq-input" hint={td.inputHint}>
          <NativeSelect
            id="rq-input"
            value={text.input ?? ""}
            placeholder={td.unset}
            options={[{ value: "mission_duration", label: td.inputMissionDuration }]}
            onChange={(e) => set("input", e.target.value)}
          />
        </Field>
      </div>
    </OpDialog>
  );
}
