import { hasContent, type KpiSlot, kpiText } from "@/analysis/kpi";
import type {
  BudgetKind,
  BudgetPropellant,
  BudgetTotal,
  BudgetView,
  FillRatio,
  KpiKey,
  KpiTally,
} from "@/api/types";
import { type Dict, localized } from "@/i18n";
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

/** バジェットの問い合わせの今 */
export interface BudgetState {
  data: BudgetView | undefined;
  pending: boolean;
}

/** カードの材料: 解析の一覧に沿った KPI の引き当てと、バジェットの問い合わせ */
export interface OverviewSources {
  /** 概要に使う解析の KPI（解析の一覧の順。`kpiSlots`） */
  slots: readonly KpiSlot[];
  budget: (kind: BudgetKind) => BudgetState;
}

/**
 * 概要の KPI カード。質量余裕はバジェット、見出しにする KPI は API が印を付けたもの（`KpiDef.overview`。並びは `order`）。
 * 値・判定・塗りの比・判定の数え方はすべて API のもので、ここは並べて文言を選ぶだけ（代表値を選んだり割ったりしない）。
 * 解析が走り終えても見せるものが無い KPI（電池やホイールの無い設計など）のカードは出さない。
 */
export function overviewCards(o: Dict["overview"], s: OverviewSources): OverviewCard[] {
  const slotOf = (key: KpiKey) => s.slots.find((x) => x.def.key === key);
  const range = (t: KpiTally) =>
    t.checked === 0 ? "—" : t.flagged === 0 ? o.allInRange : o.nodesToCheck(t.flagged);
  /** フッター: 同じ解析の別の KPI を「名前 値」で並べる */
  const footerOf = (footer: readonly KpiKey[]) =>
    footer
      .flatMap((key) => {
        const f = slotOf(key);
        return f ? [`${localized(f.def.label, f.def.label_en)} ${kpiText(f.kpi, () => "—")}`] : [];
      })
      .join(" / ");
  const kpiCard = (slot: KpiSlot): OverviewCard => {
    // API が「バジェットを添える」と印を付けた KPI（電力収支）は、塗りとフッターにそのバジェットの合計と上限を添える
    const kind = slot.def.overview?.budget;
    const attached = kind ? s.budget(kind) : null;
    return {
      key: slot.def.key,
      label: localized(slot.def.label, slot.def.label_en),
      value: kpiText(slot.kpi, range),
      fill: attached ? (attached.data?.fill ?? null) : (slot.kpi?.fill ?? null),
      tone: verdictTone[slot.kpi?.verdict ?? "ok"],
      footer: attached
        ? attached.data?.total
          ? o.consumption(attached.data.total.display)
          : ""
        : footerOf(slot.def.overview?.footer ?? []),
      footerRight: attached?.data?.limit ? o.generationLimit(attached.data.limit.display) : "",
      to: paths.area(slot.area),
      loading: slot.loading,
    };
  };
  const mass = s.budget("mass");
  const cards = mass.pending || mass.data?.total ? [massCard(o, mass)] : [];
  const heads = s.slots
    .filter((slot) => slot.def.overview)
    .sort((a, b) => (a.def.overview?.order ?? 0) - (b.def.overview?.order ?? 0));
  for (const slot of heads) {
    if (!slot.loading && !hasContent(slot.kpi)) continue;
    cards.push(kpiCard(slot));
  }
  return cards;
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
