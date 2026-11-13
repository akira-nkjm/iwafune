import { useState } from "react";
import { collectSeries } from "@/analysis/output";
import { SeriesChart } from "@/analysis/SeriesChart";
import type { AnalysisRunResult, DesignView } from "@/api/types";
import { useT } from "@/i18n";
import { Button } from "@/ui/button";
import { EclipseCaseDialog } from "./EclipseCaseDialog";

/** 物理計算・代表日の選択は API に任せ、系列とケース作成の入口を表示する。 */
export function EclipseSeasonPanel({
  result,
  design,
}: {
  result: AnalysisRunResult;
  design: DesignView | undefined;
}) {
  const m = useT().conditions;
  const [at, setAt] = useState<string | null>(null);
  return (
    <div className="space-y-4">
      <p className="text-ink-3 text-xs">{m.eclipseApproximation}</p>
      {collectSeries(result.output).map((s) => (
        <SeriesChart key={s.path} series={s.series} title={m.eclipseSeries} />
      ))}
      <div className="flex flex-wrap gap-2">
        <Button variant="outline" onClick={() => setAt("longest")}>
          {m.eclipseLongest} · {m.createEclipseCase}
        </Button>
        <Button variant="outline" onClick={() => setAt("beta_min")}>
          {m.eclipseBetaMin} · {m.createEclipseCase}
        </Button>
      </div>
      {at !== null && <EclipseCaseDialog at={at} onClose={() => setAt(null)} design={design} />}
    </div>
  );
}
