import type { AnalysisRunResult, DesignView } from "@/api/types";
import { localized, useT } from "@/i18n";
import { VerdictPill } from "@/kit/status/VerdictPill";
import { ChecksList } from "./ChecksList";
import { AnalysisDetail, hasDetailView } from "./detail/AnalysisDetail";
import { fieldLabel } from "./fields";
import { kpiText } from "./kpi";
import { MissingList } from "./MissingList";
import { OverviewBars } from "./OverviewBars";
import { collectTables, type FoundTable } from "./output";
import { ResultTable } from "./ResultTable";

/** 表の位置が隠す表（`hidden_tables`。キーの列）のどれかと同じか（区間ごとに比べる。文字列を分けない） */
export function isHiddenTable(table: Pick<FoundTable, "at">, hidden: readonly string[][]): boolean {
  return hidden.some((h) => h.length === table.at.length && h.every((k, i) => k === table.at[i]));
}

/**
 * 解析の表の行を開いたところ: KPI の全部（名前・値・判定）・小さな棒グラフ・持ち主ごとの判定の文・足りない入力と、
 * 畳んだ「データを見る」（生の表。ほかの行と同じ表は API の `hidden_tables` で省く）。どの系の解析でも同じ形。
 * 判定のまとまり・棒グラフ・隠す表は結果に API が添えた概要（`result.overview`）。
 */
export function AnalysisRowDetail({
  result,
  design,
  detailType,
}: {
  result: AnalysisRunResult;
  design?: DesignView;
  /** 解析が持つ重い任意の出力の型名（`AnalysisEntry.detail_type`）。表示があれば、開いたときに取って出す */
  detailType?: string;
}) {
  const dict = useT();
  const overview = result.overview;
  const t = dict.systems.analysisTable;
  const tables = result.output
    ? collectTables(result.output).filter((x) => !isHiddenTable(x, overview.hidden_tables))
    : [];
  return (
    <div className="space-y-4">
      {result.error && <p className="text-neg-ink text-sm">{result.error}</p>}
      <MissingList design={design} missing={result.missing} />
      {(result.kpis.length > 0 || overview.chart) && (
        <div className="grid gap-4 lg:grid-cols-[minmax(0,18rem)_minmax(0,1fr)]">
          {result.kpis.length > 0 && (
            <section aria-label={t.kpis}>
              <h3 className="mb-2 font-medium text-ink-2 text-xs">{t.kpis}</h3>
              <dl className="divide-y divide-line-soft rounded-control border border-line">
                {result.kpis.map((k) => (
                  <div key={k.key} className="flex items-center gap-2 px-3 py-1.5 text-xs">
                    <dt className="min-w-0 flex-1 text-ink-2">{localized(k.label, k.label_en)}</dt>
                    <dd className="num shrink-0 text-right font-medium text-ink">
                      {kpiText(k, (n) => dict.systems.kpiTally(n.checked, n.flagged))}
                    </dd>
                    {k.verdict && (
                      <span className="shrink-0">
                        <VerdictPill verdict={k.verdict} />
                      </span>
                    )}
                  </div>
                ))}
              </dl>
            </section>
          )}
          {overview.chart && <OverviewBars chart={overview.chart} />}
        </div>
      )}
      {overview.checks.length > 0 && (
        <section aria-label={t.checks} className="space-y-2">
          <h3 className="font-medium text-ink-2 text-xs">{t.checks}</h3>
          {overview.checks.map((g) => (
            <div key={g.scope ?? ""} className="space-y-1">
              {g.scope && <h4 className="text-ink-3 text-xs">{g.scope}</h4>}
              <ChecksList checks={g.checks} showScope={false} />
            </div>
          ))}
        </section>
      )}
      {detailType !== undefined && hasDetailView(detailType) && (
        <AnalysisDetail result={result} detailType={detailType} />
      )}
      {tables.length > 0 && (
        <details>
          <summary className="cursor-pointer text-ink-3 text-sm">{dict.systems.data}</summary>
          <div className="mt-3 space-y-4">
            {tables.map((table) => (
              <div key={table.path} className="overflow-auto">
                <h3 className="mb-2 font-medium text-sm">{fieldLabel(table.key)}</h3>
                <ResultTable
                  rows={table.rows}
                  at={table.at}
                  dims={result.dims}
                  labels={result.labels}
                />
              </div>
            ))}
          </div>
        </details>
      )}
    </div>
  );
}
