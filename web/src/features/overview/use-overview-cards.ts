import { useQuery } from "@tanstack/react-query";
import { analysesFor, kpiSlots, kpiText } from "@/analysis/kpi";
import { q, useAnalyses } from "@/api/queries";
import { localized, useT } from "@/i18n";
import { verdictTone } from "@/kit/status/verdict-tone";
import { paths } from "@/lib/nav";
import { OVERVIEW_KPIS, type OverviewCard, overviewCards } from "./cards";

/**
 * 概要の KPI カード。KPI を出す解析を解析の一覧から選んで実行し（系のページと同じキーでキャッシュに入る）、
 * 質量・電力のバジェットと合わせて `overviewCards` に並べさせる。
 */
export function useOverviewCards(): { cards: OverviewCard[]; conditions: OverviewCard[] } {
  const o = useT().overview;
  const catalog = useQuery(q.analyses());
  const entries = catalog.data ?? [];
  const ids = [
    ...new Set([
      ...analysesFor(entries, OVERVIEW_KPIS),
      ...entries
        .filter((e) => e.info.area.kind === "condition" && e.kpis.length > 0)
        .map((e) => e.info.id),
    ]),
  ];
  const states = useAnalyses(ids, null);
  const slots = kpiSlots(
    entries.filter((e) => ids.includes(e.info.id)),
    states,
  );
  const mass = useQuery(q.budget("mass"));
  const power = useQuery(q.budget("power"));
  const cards = overviewCards(o, {
    kpi: (key) => slots.find((s) => s.def.key === key),
    mass: { data: mass.data, pending: mass.isPending },
    power: { data: power.data, pending: power.isPending },
  });
  const conditions = slots
    .filter((s) => s.area.kind === "condition")
    .map(
      (s): OverviewCard => ({
        key: s.def.key,
        label: localized(s.def.label, s.def.label_en),
        value: kpiText(s.kpi, () => "—"),
        fill: s.kpi?.fill ?? null,
        tone: verdictTone[s.kpi?.verdict ?? "ok"],
        footer: "",
        footerRight: "",
        to: paths.area(s.area),
        loading: s.loading,
      }),
    );
  return { cards, conditions };
}
