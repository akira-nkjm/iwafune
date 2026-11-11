import { RefreshCw } from "lucide-react";
import { recomputeAnalyses } from "@/api/queries";
import type { CaseView } from "@/api/types";
import { useT } from "@/i18n";
import { NativeSelect } from "@/kit/forms/NativeSelect";
import { idWithName } from "@/lib/design";
import { Button } from "@/ui/button";

/** 系のページの右上: 解析ケースの選択と「再計算」 */
export function SystemHeaderActions({
  cases,
  kase,
  defaultCaseId,
  onCase,
  ids,
}: {
  cases: CaseView[];
  kase: string | null;
  defaultCaseId: string | null;
  onCase: (c: string | null) => void;
  ids: string[];
}) {
  const t = useT();
  const defaultCase = cases.find((c) => c.id === defaultCaseId);
  const placeholder = defaultCaseId
    ? t.systems.defaultDerivedCase(defaultCase ? idWithName(defaultCase) : defaultCaseId)
    : t.systems.defaultCase;
  return (
    <>
      <NativeSelect
        aria-label={t.systems.caseLabel}
        value={kase ?? ""}
        placeholder={placeholder}
        onChange={(e) => onCase(e.target.value || null)}
        options={cases.map((c) => ({ value: c.id, label: idWithName(c) }))}
      />
      <Button variant="outline" onClick={() => recomputeAnalyses(ids, kase)}>
        <RefreshCw size={14} />
        {t.systems.recompute}
      </Button>
    </>
  );
}
