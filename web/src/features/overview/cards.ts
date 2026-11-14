import { type KpiSlot, kpiText } from "@/analysis/kpi";
import type {
  BudgetPropellant,
  BudgetTotal,
  BudgetView,
  FillRatio,
  KpiKey,
  KpiTally,
} from "@/api/types";
import type { Dict } from "@/i18n";
import type { TickTone } from "@/kit/metrics/TickBar";
import { verdictTone } from "@/kit/status/verdict-tone";
import { paths } from "@/lib/nav";

/** 概要の KPI カード 1 枚（`kit/metrics/KpiCard` に渡すもの） */
export interface OverviewCard {
  key: string;
  label: string;
  value: string;
  /** 塗りの比（API の `fill`）。null は測れるものが無い */
  fill: FillRatio | null;
  tone: TickTone;
  footer: string;
  footerRight: string;
  /** 押したときに開く系のページ */
  to: string;
  loading: boolean;
}

/**
 * 概要のカードに使う KPI（カードの値・塗り・判定と、フッターに添える値）。どの解析が出すかは解析の一覧
 * （`AnalysisEntry.kpis`）から引くので、ここには解析 ID を書かない。
 */
export const OVERVIEW_KPIS: readonly KpiKey[] = [
  "orbit_balance",
  "max_dod",
  "unload_interval",
  "peak_momentum",
  "storage_margin",
  "data_per_day",
  "link_margin",
  "nodes_in_range",
  "max_temperature",
  "min_temperature",
];

/** バジェットの問い合わせの今 */
export interface BudgetState {
  data: BudgetView | undefined;
  pending: boolean;
}

/** カードの材料: KPI の引き当て（キー → 値と解析の状態）と、質量・電力のバジェット */
export interface OverviewSources {
  kpi: (key: KpiKey) => KpiSlot | undefined;
  mass: BudgetState;
  power: BudgetState;
}

/**
 * 概要の KPI カード（質量余裕はバジェット、電力収支は解析の KPI と電力のバジェット、残りは系の解析の KPI）。
 * 値・判定・塗りの比・判定の数え方はすべて API のもので、ここは並べて文言を選ぶだけ（代表値を選んだり割ったりしない）。
 */
export function overviewCards(o: Dict["overview"], s: OverviewSources): OverviewCard[] {
  const text = (key: KpiKey) => kpiText(s.kpi(key)?.kpi, () => "—");
  const kpiCard = (
    key: KpiKey,
    label: string,
    rest: Partial<Pick<OverviewCard, "value" | "fill" | "footer" | "footerRight">> = {},
  ): OverviewCard => {
    const slot = s.kpi(key);
    return {
      key,
      label,
      value: rest.value ?? text(key),
      fill: rest.fill === undefined ? (slot?.kpi?.fill ?? null) : rest.fill,
      tone: verdictTone[slot?.kpi?.verdict ?? "ok"],
      footer: rest.footer ?? "",
      footerRight: rest.footerRight ?? "",
      to: slot ? paths.area(slot.area) : paths.overview(),
      loading: slot?.loading ?? true,
    };
  };
  const range = (t: KpiTally) =>
    t.checked === 0 ? "—" : t.flagged === 0 ? o.allInRange : o.nodesToCheck(t.flagged);
  const { mass, power } = s;
  return [
    massCard(o, mass),
    kpiCard("orbit_balance", o.powerBalance, {
      fill: power.data?.fill ?? null,
      footer: power.data?.total ? o.consumption(power.data.total.display) : "",
      footerRight: power.data?.limit ? o.generationLimit(power.data.limit.display) : "",
    }),
    kpiCard("max_dod", o.maxDod, { footer: o.dodFooter }),
    kpiCard("unload_interval", o.rwInterval, {
      footer: `${o.peakMomentum} ${text("peak_momentum")}`,
    }),
    kpiCard("storage_margin", o.storageMargin, {
      footer: `${o.perDay} ${text("data_per_day")}`,
    }),
    kpiCard("link_margin", o.linkMargin),
    kpiCard("nodes_in_range", o.temperature, {
      value: kpiText(s.kpi("nodes_in_range")?.kpi, range),
      footer: `${o.tempMax} ${text("max_temperature")} / ${o.tempMin} ${text("min_temperature")}`,
    }),
  ];
}

/** 推薬込み（wet）の合計（質量のバジェットが推薬を数えたときだけ） */
export function wetTotal(p: BudgetPropellant | null | undefined): BudgetTotal | null {
  return p?.status === "counted" ? p.wet : null;
}

/**
 * 質量余裕のカード。推薬を数えたときは推薬込み（打ち上げの質量）の余裕・判定・塗りを出し、フッターに乾燥と推薬込みの
 * 両方の合計を添える。数えていなければ乾燥の値（値と判定は API のものを並べるだけ）。
 */
function massCard(o: Dict["overview"], mass: BudgetState): OverviewCard {
  const b = mass.data;
  const wet = wetTotal(b?.propellant);
  const verdict = wet ? wet.verdict : b?.verdict;
  const dry = b?.total?.display;
  return {
    key: "mass",
    label: o.massMargin,
    value: (wet ? wet.margin : b?.margin)?.display ?? "—",
    fill: (wet ? wet.fill : b?.fill) ?? null,
    tone: verdict ? verdictTone[verdict] : "accent",
    footer: dry ? (wet ? o.massDryWet(dry, wet.total.display) : o.massTotal(dry)) : o.massBudget,
    footerRight: b?.limit ? o.limit(b.limit.display) : "",
    to: b ? paths.system(b.system) : paths.overview(),
    loading: mass.pending,
  };
}
