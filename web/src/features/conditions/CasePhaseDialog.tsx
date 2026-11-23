import { useState } from "react";
import { useCommandOps } from "@/api/ops";
import type { CaseView, PhaseView } from "@/api/types";
import { useT } from "@/i18n";
import { Field } from "@/kit/forms/Field";
import { NativeSelect } from "@/kit/forms/NativeSelect";
import { OpDialog } from "@/ops/OpDialog";
import { useOpCheck } from "@/ops/use-op-check";
import { casePhaseCommand } from "./case-phase";

/**
 * 解析ケースのフェーズの選択（`fune case set-phase|unset-phase` と同じ引数を service に送り、Op にしてから確定する）。
 * 選んだフェーズの軌道と姿勢則の上書きが、そのケースの解析に使われる。空を選べば外す。
 */
export function CasePhaseDialog({
  kase,
  phases,
  onOpenChange,
}: {
  kase: CaseView;
  phases: PhaseView[];
  onOpenChange: (open: boolean) => void;
}) {
  const t = useT().conditions.casePhase;
  const [phase, setPhase] = useState(kase.phase ?? "");
  const made = useCommandOps(casePhaseCommand(kase.id, phase));
  const error = useOpCheck(made.ops).error ?? made.error;
  return (
    <OpDialog
      open
      onOpenChange={onOpenChange}
      title={t.title(kase.id)}
      description={t.description}
      ops={made.ops}
      summary={t.summary(kase.id, phase)}
      confirmLabel={t.confirm}
    >
      <Field label={t.field} htmlFor="case-phase" error={error}>
        <NativeSelect
          id="case-phase"
          value={phase}
          placeholder={t.none}
          options={phases.map((p) => ({ value: p.id, label: `${p.id} — ${p.label}` }))}
          onChange={(e) => setPhase(e.target.value)}
        />
      </Field>
    </OpDialog>
  );
}
