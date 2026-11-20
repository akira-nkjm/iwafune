import type {
  ThermalRangeKind,
  ThermalRangeMargin,
  ThermalSteadyOutput,
  ThermalTransientOutput,
  Verdict,
} from "@/api/types";

/** 範囲の種類の並び（外側 = 保管、内側 = 性能。API の `ranges` の並びと同じ） */
export const RANGE_KINDS: readonly ThermalRangeKind[] = ["storage", "operating", "performance"];

/** 1 つの節点の 1 種類の温度範囲の帯（K） */
export interface RangeBand {
  kind: ThermalRangeKind;
  min: number;
  max: number;
}

/** 定常のケース 1 つの印。判定と余裕は API の値をそのまま持つ（ここでは計算しない） */
export interface RangeMarker {
  caseId: string;
  label: string;
  temperature: number;
  verdict: Verdict;
  ranges: ThermalRangeMargin[];
}

/** 非定常の最低〜最高の棒 */
export interface RangeSpan {
  min: number;
  max: number;
  verdict: Verdict;
  ranges: ThermalRangeMargin[];
}

/** 節点 1 行分 */
export interface RangeRow {
  node: string;
  bands: RangeBand[];
  markers: RangeMarker[];
  span: RangeSpan | null;
}

/** 範囲の図の元データ。`lo`・`hi` は横軸の端（K。データと帯を含み、少し余白を足したもの） */
export interface RangeChartModel {
  rows: RangeRow[];
  caseIds: string[];
  lo: number;
  hi: number;
}

/** 軸の端に足す余白（幅の割合）。印が端に張り付かないように */
const PAD = 0.04;

/** 節点の帯: 各種類について最初に見つかった範囲を使う（範囲はケースで変わらないが、モードで選び直されることがある） */
function bandsOf(lists: readonly (readonly ThermalRangeMargin[])[]): RangeBand[] {
  const by = new Map<ThermalRangeKind, RangeBand>();
  for (const list of lists)
    for (const r of list)
      if (!by.has(r.kind)) by.set(r.kind, { kind: r.kind, min: r.range.min, max: r.range.max });
  return RANGE_KINDS.flatMap((k) => by.get(k) ?? []);
}

/**
 * thermal.steady（ケースごとの節点）と thermal.transient（節点ごとの最低・最高）から、節点 1 行ずつの図の元データを作る。
 * 行の並びは定常の節点の順、非定常にだけある節点はその後ろ。どちらの出力も無ければ null。値は SI（K）のまま。
 */
export function rangeChartOf(
  steady: ThermalSteadyOutput | null,
  transient: ThermalTransientOutput | null,
): RangeChartModel | null {
  const rows = new Map<string, RangeRow & { lists: ThermalRangeMargin[][] }>();
  const row = (node: string) => {
    let r = rows.get(node);
    if (!r) {
      r = { node, bands: [], markers: [], span: null, lists: [] };
      rows.set(node, r);
    }
    return r;
  };
  const caseIds: string[] = [];
  for (const c of steady?.cases ?? []) {
    caseIds.push(c.id);
    for (const n of c.nodes) {
      const r = row(n.node);
      r.markers.push({
        caseId: c.id,
        label: c.label || c.id,
        temperature: n.temperature,
        verdict: n.verdict,
        ranges: n.ranges,
      });
      r.lists.push(n.ranges);
    }
  }
  for (const n of transient?.nodes ?? []) {
    const r = row(n.node);
    r.span = { min: n.min, max: n.max, verdict: n.verdict, ranges: n.ranges };
    r.lists.push(n.ranges);
  }
  if (rows.size === 0) return null;
  let lo = Number.POSITIVE_INFINITY;
  let hi = Number.NEGATIVE_INFINITY;
  const out: RangeRow[] = [];
  for (const { lists, ...r } of rows.values()) {
    const bands = bandsOf(lists);
    const values = [
      ...r.markers.map((m) => m.temperature),
      ...(r.span ? [r.span.min, r.span.max] : []),
      ...bands.flatMap((b) => [b.min, b.max]),
    ];
    for (const v of values) {
      lo = Math.min(lo, v);
      hi = Math.max(hi, v);
    }
    out.push({ ...r, bands });
  }
  const pad = (hi - lo || 1) * PAD;
  return { rows: out, caseIds, lo: lo - pad, hi: hi + pad };
}

/** 値の横軸の位置（0〜1。範囲の外は端に丸める）。図に置く位置を決めるだけの写像 */
export function axisPosition(v: number, lo: number, hi: number): number {
  return hi > lo ? Math.min(1, Math.max(0, (v - lo) / (hi - lo))) : 0.5;
}

/** 横軸の目盛りの位置（K。端を含めて等間隔に `n` 個。表示の単位と桁は呼び手が API で整える） */
export function axisTicks(lo: number, hi: number, n = 5): number[] {
  return Array.from({ length: n }, (_, i) => lo + ((hi - lo) * i) / (n - 1));
}
