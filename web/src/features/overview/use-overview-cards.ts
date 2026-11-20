import { useQueries, useQuery } from "@tanstack/react-query";
import { analysesFor, kpiSlots, kpiText, overviewKeys } from "@/analysis/kpi";
import { q, useAnalyses, useDesign } from "@/api/queries";
import { localized, useT } from "@/i18n";
import { verdictTone } from "@/kit/status/verdict-tone";
import { paths } from "@/lib/nav";
import { type OverviewCard, overviewCards } from "./cards";

/**
 * 概要の KPI カード。KPI を出す解析を、API が印を付けた KPI（`KpiDef.overview`）から選んで実行し（系のページと同じキーで
 * キャッシュに入る）、バジェット（種類は `DefsView.budget_kinds`）と合わせて `overviewCards` に並べさせる。
 */
export function useOverviewCards(): { cards: OverviewCard[]; conditions: OverviewCard[] } {
  const o = useT().overview;
  const catalog = useQuery(q.analyses());
  const entries = catalog.data ?? [];
  const kinds = useDesign().data?.defs.budget_kinds ?? [];
  const ids = [
    ...new Set([
      ...analysesFor(entries, overviewKeys(entries)),
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
  const budgets = useQueries({
    queries: kinds.map((kind) => q.budget(kind)),
    combine: (rs) =>
      new Map(
        kinds.map((kind, i) => [kind, { data: rs[i]?.data, pending: rs[i]?.isPending ?? true }]),
      ),
  });
  const cards = overviewCards(o, {
    slots,
    budget: (kind) => budgets.get(kind) ?? { data: undefined, pending: true },
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
