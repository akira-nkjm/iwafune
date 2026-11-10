import { useQuery } from "@tanstack/react-query";
import { analysesFor, kpiSlots } from "@/analysis/kpi";
import { q, useAnalyses } from "@/api/queries";
import { useT } from "@/i18n";
import { OVERVIEW_KPIS, type OverviewCard, overviewCards } from "./cards";

/**
 * 概要の KPI カード。KPI を出す解析を解析の一覧から選んで実行し（系のページと同じキーでキャッシュに入る）、
 * 質量・電力のバジェットと合わせて `overviewCards` に並べさせる。
 */
export function useOverviewCards(): OverviewCard[] {
  const o = useT().overview;
  const catalog = useQuery(q.analyses());
  const entries = catalog.data ?? [];
  const ids = analysesFor(entries, OVERVIEW_KPIS);
  const states = useAnalyses(ids, null);
  const slots = kpiSlots(
    entries.filter((e) => ids.includes(e.info.id)),
    states,
  );
  const mass = useQuery(q.budget("mass"));
  const power = useQuery(q.budget("power"));
  return overviewCards(o, {
    kpi: (key) => slots.find((s) => s.def.key === key),
    mass: { data: mass.data, pending: mass.isPending },
    power: { data: power.data, pending: power.isPending },
  });
}
