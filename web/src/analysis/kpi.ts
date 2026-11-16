import type { AnalysisState } from "@/api/queries";
import type { AnalysisArea, AnalysisEntry, KpiDef, KpiKey, KpiTally, KpiView } from "@/api/types";

/**
 * KPI を並べるための引き当て。**代表値の選び方・判定・塗りの比・判定の数え方は service が決める**
 * （`fune_service::analysis::kpi`）。ここは解析の一覧（`AnalysisEntry.kpis`）と実行の結果（`AnalysisRunResult.kpis`）を
 * キーで突き合わせるだけで、値を計算しない。
 */

/** 並べる KPI 1 つ: 名前（定義）・出す解析と系・届いた値 */
export interface KpiSlot {
  def: KpiDef;
  /** KPI を出す解析の ID */
  analysis: string;
  /** その解析の区分（開き先のページ） */
  area: AnalysisArea;
  /** 結果の KPI（まだ届いていないか、解析が出せなかったら null） */
  kpi: KpiView | null;
  /** 解析の結果を待っている */
  loading: boolean;
}

/** 解析の一覧の順に、それぞれの解析の KPI を並べる（系のページの数値帯）。値は結果の `kpis` をキーで引く */
export function kpiSlots(
  entries: readonly AnalysisEntry[],
  states: readonly AnalysisState[],
): KpiSlot[] {
  return entries.flatMap((e) => {
    const state = states.find((s) => s.id === e.info.id);
    const delivered = state?.result?.kpis ?? [];
    return (
      e.kpis
        // 結果が届いていて、測れたときだけ出す KPI（設計が持たない手段）が無いときは並べない
        .filter(
          (def) =>
            !def.when_measured ||
            delivered.length === 0 ||
            delivered.some((k) => k.key === def.key),
        )
        .map((def) => ({
          def,
          analysis: e.info.id,
          area: e.info.area,
          kpi: delivered.find((k) => k.key === def.key) ?? null,
          loading: state?.loading ?? true,
        }))
    );
  });
}

/** KPI のキーを出す解析（解析の一覧の順・重ねない）。概要で実行する解析を選ぶのに使う */
export function analysesFor(entries: readonly AnalysisEntry[], keys: readonly KpiKey[]): string[] {
  return entries.filter((e) => e.kpis.some((k) => keys.includes(k.key))).map((e) => e.info.id);
}

/**
 * KPI の値の文字: 値があれば API の整形（`display`）、判定を数える KPI は `tally` の文（言い方は呼び手の i18n）、
 * どちらも無ければ「—」
 */
export function kpiText(kpi: KpiView | null | undefined, tally: (t: KpiTally) => string): string {
  if (kpi?.value) return kpi.value.display;
  if (kpi?.tally) return tally(kpi.tally);
  return "—";
}
