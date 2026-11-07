import { RefreshCw } from "lucide-react";
import { recomputeAnalyses } from "@/api/queries";
import type { CaseView } from "@/api/types";
import { useT } from "@/i18n";
import { NativeSelect } from "@/kit/NativeSelect";
import { Button } from "@/ui/button";

/** 系のページの右上: 解析ケースの選択と「再計算」 */
export function SystemHeaderActions({
  cases,
  kase,
  onCase,
  ids,
}: {
  cases: CaseView[];
  kase: string | null;
  onCase: (c: string | null) => void;
  ids: string[];
}) {
  const t = useT();
  return (
    <>
      <NativeSelect
        aria-label={t.systems.caseLabel}
        value={kase ?? ""}
        placeholder={t.systems.defaultCase}
        onChange={(e) => onCase(e.target.value || null)}
        options={cases.map((c) => ({ value: c.id, label: t.systems.caseOption(c.label, c.id) }))}
      />
      <Button variant="outline" onClick={() => recomputeAnalyses(ids, kase)}>
        <RefreshCw size={14} />
        {t.systems.recompute}
      </Button>
    </>
  );
}
