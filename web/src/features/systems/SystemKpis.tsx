import { kpiSlots, kpiText } from "@/analysis/kpi";
import type { AnalysisState } from "@/api/queries";
import type { AnalysisEntry } from "@/api/types";
import { localized, useT } from "@/i18n";
import { Loading, SkeletonBlock } from "@/kit/feedback/Loading";
import { StatStrip } from "@/kit/metrics/StatStrip";

/**
 * 系のページ上部の主要 KPI。並び・名前・値は API（解析の一覧の `kpis` と、結果の `kpis`）で、ここは並べるだけ。
 * 解析の一覧を待つ間は帯のスケルトン、結果を待つ間は値のスケルトン。
 */
export function SystemKpis({
  entries,
  states,
  loading,
}: {
  /** この系の解析（一覧の順） */
  entries: readonly AnalysisEntry[];
  states: readonly AnalysisState[];
  /** 解析の一覧を待っている */
  loading: boolean;
}) {
  const t = useT().systems;
  if (loading)
    return (
      <Loading>
        <SkeletonBlock className="h-[62px] rounded-card" />
      </Loading>
    );
  return (
    <StatStrip
      items={kpiSlots(entries, states).map((s) => ({
        key: s.def.key,
        label: localized(s.def.label, s.def.label_en),
        value: kpiText(s.kpi, (n) => t.kpiTally(n.checked, n.flagged)),
        loading: s.loading,
      }))}
    />
  );
}
