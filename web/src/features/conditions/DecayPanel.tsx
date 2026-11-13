import { collectSeries, jsonPointer } from "@/analysis/output";
import { ResultTable } from "@/analysis/ResultTable";
import { SeriesChart } from "@/analysis/SeriesChart";
import { analysisOutput } from "@/api/analysis-output";
import type { AnalysisRunResult } from "@/api/types";
import { localized, useT } from "@/i18n";

/** 高度低下（orbit.decay）: 条件ごとの表と高度の推移。計算・判定は API に任せ、使った宇宙天気の表の版を添える。 */
export function DecayPanel({ result }: { result: AnalysisRunResult }) {
  const m = useT().conditions;
  const out = analysisOutput(result, "orbit.decay");
  if (!out) return null;
  const caseName = (i: number): string => {
    const label =
      result.labels[jsonPointer(["cases", i, "condition", "activity"])] ??
      result.labels[jsonPointer(["cases", i, "condition", "kind"])];
    return label ? localized(label.label, label.label_en) : (out.cases[i]?.id ?? "");
  };
  const version = result.space_weather;
  return (
    <div className="space-y-4">
      <p className="text-ink-3 text-xs">{m.decayApproximation}</p>
      <p className="text-ink-3 text-xs">
        {version
          ? m.decayWeatherTable(version.first_date, version.last_date, version.fetched_at ?? "—")
          : m.decayNoWeatherTable}
      </p>
      <ResultTable rows={out.cases} at={["cases"]} dims={result.dims} labels={result.labels} />
      <div className="grid gap-4 xl:grid-cols-2">
        {out.cases.map((c, i) =>
          collectSeries(c.series).map((s) => (
            <SeriesChart
              key={`${c.id}${s.path}`}
              series={s.series}
              title={`${m.decaySeries} · ${caseName(i)}`}
            />
          )),
        )}
      </div>
    </div>
  );
}
