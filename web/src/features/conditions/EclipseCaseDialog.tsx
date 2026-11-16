import { useId, useState } from "react";
import { useEclipseCaseOp } from "@/api/ops";
import type { DesignView } from "@/api/types";
import { useT } from "@/i18n";
import { OpDialog } from "@/ops/OpDialog";
import { Input } from "@/ui/input";

/** 季節解析のケース作成。Op のプレビューと保存は共通のダイアログに任せる。 */
export function EclipseCaseDialog({
  at,
  onClose,
  design,
}: {
  at: string;
  onClose: () => void;
  design: DesignView | undefined;
}) {
  const m = useT().conditions;
  const fieldId = useId();
  const [id, setId] = useState("");
  const [mode, setMode] = useState("");
  const built = useEclipseCaseOp(id, at, mode);
  const title = at === "beta_min" ? m.eclipseBetaMin : m.eclipseLongest;
  return (
    <OpDialog
      open
      onOpenChange={(open) => {
        if (!open) onClose();
      }}
      title={title}
      description={m.eclipseCaseDescription}
      ops={built.ops}
      summary={`${m.createEclipseCase}: ${id}`}
      confirmLabel={m.createEclipseCase}
    >
      <label className="grid gap-1 text-xs" htmlFor={`${fieldId}-id`}>
        {m.caseId}
        <Input id={`${fieldId}-id`} value={id} onChange={(e) => setId(e.target.value)} />
      </label>
      <label className="grid gap-1 text-xs" htmlFor={`${fieldId}-mode`}>
        {m.caseMode}
        <select
          id={`${fieldId}-mode`}
          className="rounded border border-line bg-surface p-2"
          value={mode}
          onChange={(e) => setMode(e.target.value)}
        >
          <option value="">{m.noCaseMode}</option>
          {design?.modes.modes.map((m) => (
            <option key={m.id} value={m.id}>
              {m.label}
            </option>
          ))}
        </select>
      </label>
      {built.error && <p className="text-neg-ink text-xs">{built.error}</p>}
    </OpDialog>
  );
}
