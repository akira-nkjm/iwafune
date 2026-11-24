import type { PartHeat, ThermalSteadyOutput, ThermalTransientOutput } from "@/api/types";
import { type HeatScale, heatRatio, heatScaleOfValues } from "@/lib/heat-scale";
import { heatHex, type Scheme, toneHexFor } from "@/lib/tone";

/** 熱画像に使う温度の出どころ: 全ケースの最悪（API の `part_heat`）・定常の 1 ケース・非定常のある時刻 */
export type HeatSource =
  | { kind: "worst" }
  | { kind: "case"; id: string }
  | { kind: "transient"; index: number };

/** 非定常の節点ごとの温度の時系列（API の `series` から温度の系列だけ取り出したもの） */
export interface TransientTemperatures {
  /** 経過時間 [s] */
  t: number[];
  /** 節点 ID → 温度 [K]（t と同じ長さ） */
  nodes: Map<string, number[]>;
}

/** thermal.transient の出力から節点ごとの温度の時系列を取り出す（温度の次元の系列だけ。値は SI のまま） */
export function transientTemperatures(
  output: ThermalTransientOutput | null,
): TransientTemperatures | null {
  if (!output || output.series.t.length === 0) return null;
  const nodes = new Map<string, number[]>();
  for (const c of output.series.channels)
    if (c.kind === "node_temperature") nodes.set(c.subject ?? c.key, c.values);
  return nodes.size === 0 ? null : { t: output.series.t, nodes };
}

/**
 * 色の範囲（`lib/heat-scale.ts`）。全ケースの定常の節点温度と、非定常の全時刻の節点温度の最低・最高。出どころを切り替えても
 * 同じ範囲を使うので、色の意味は変わらない。値が 1 つも無ければ null。
 */
export function heatScaleOf(
  steady: ThermalSteadyOutput | null,
  transient: TransientTemperatures | null,
): HeatScale | null {
  const values: number[] = [];
  for (const c of steady?.cases ?? []) for (const n of c.nodes) values.push(n.temperature);
  for (const series of transient?.nodes.values() ?? []) values.push(...series);
  return heatScaleOfValues(values);
}

/** 選んだ出どころでの節点の温度（節点 ID → K）。最悪は部品ごとの代表温度（`part_heat`）から節点ごとに拾う */
export function nodeTemperatures(
  source: HeatSource,
  steady: ThermalSteadyOutput | null,
  transient: TransientTemperatures | null,
  heat: readonly PartHeat[] | undefined,
): Map<string, number> {
  const out = new Map<string, number>();
  if (source.kind === "worst") {
    for (const h of heat ?? []) out.set(h.node, h.temperature.si);
  } else if (source.kind === "case") {
    const c = steady?.cases.find((x) => x.id === source.id);
    for (const n of c?.nodes ?? []) out.set(n.node, n.temperature);
  } else if (transient) {
    const i = Math.min(Math.max(source.index, 0), transient.t.length - 1);
    for (const [node, values] of transient.nodes) {
      const v = values[i];
      if (v !== undefined) out.set(node, v);
    }
  }
  return out;
}

/**
 * 3D の部品の色（部品の ID → 色）。部品と節点の対応は API の `part_heat`（`PartHeat.node`）が決め、
 * ここは「その節点の温度」を選んだ出どころの値から引いて色に写すだけ。対応が無い部品と、温度の無い節点の部品は灰色。
 */
export function partHeatColors(
  heat: readonly PartHeat[] | undefined,
  temps: ReadonlyMap<string, number>,
  scale: HeatScale,
  scheme: Scheme = "light",
): (part: string) => string {
  const nodeOf = new Map((heat ?? []).map((h) => [h.part, h.node]));
  return (part) => {
    const node = nodeOf.get(part);
    const t = node === undefined ? undefined : temps.get(node);
    return t === undefined ? toneHexFor(scheme).gray : heatHex(heatRatio(t, scale));
  };
}
