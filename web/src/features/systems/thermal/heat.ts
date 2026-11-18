import type { PartHeat, ThermalSteadyOutput, Verdict } from "@/api/types";
import { heatHex, toneHex } from "@/lib/tone";

export interface HeatCell {
  temperature: number;
  verdict: Verdict | null;
}

export interface HeatMap {
  cases: { id: string; label: string }[];
  nodes: string[];
  cells: Map<string, HeatCell>;
  min: number;
  max: number;
  /** 範囲の端から何 K 以内を注意にするか（出力の margin.warning_band。温度差 [K]） */
  band: number | null;
}

export const heatKey = (kase: string, node: string) => `${kase}\u0000${node}`;

/** thermal.steady の出力（cases[].nodes[]）を「ケース × 節点」の温度表にする。値は SI（K）のまま */
export function heatMapOf(output: ThermalSteadyOutput | null): HeatMap | null {
  if (!output) return null;
  const cases: HeatMap["cases"] = [];
  const nodes: string[] = [];
  const cells = new Map<string, HeatCell>();
  let min = Number.POSITIVE_INFINITY;
  let max = Number.NEGATIVE_INFINITY;
  for (const c of output.cases) {
    cases.push({ id: c.id, label: typeof c.label === "string" && c.label ? c.label : c.id });
    for (const n of c.nodes) {
      if (!nodes.includes(n.node)) nodes.push(n.node);
      const v = n.verdict;
      cells.set(heatKey(c.id, n.node), {
        temperature: n.temperature,
        verdict: v,
      });
      min = Math.min(min, n.temperature);
      max = Math.max(max, n.temperature);
    }
  }
  if (cases.length === 0) return null;
  const band = output.margin.warning_band;
  return { cases, nodes, cells, min, max, band };
}

/**
 * 色の位置（0 = 表の最低温度、1 = 最高温度）。色を塗るためだけの比で、物理量ではない。色そのものは
 * `lib/tone.ts` の `heatHex`（温度表と 3D で共通）
 */
export function heatRatio(t: number, map: Pick<HeatMap, "min" | "max">): number {
  return map.max > map.min ? (t - map.min) / (map.max - map.min) : 0.5;
}

/**
 * 3D の部品の色（部品の ID → 色）。代表の温度（全ケースの最高）と、節点と部品の対応は API の `part_heat` が決める。
 * ここは値を色に写すだけ。`part_heat` に無い部品（節点に入っていない部品）は灰色。
 */
export function partHeatColors(
  heat: readonly PartHeat[] | undefined,
  map: Pick<HeatMap, "min" | "max">,
): (part: string) => string {
  const by = new Map((heat ?? []).map((h) => [h.part, h]));
  return (part) => {
    const h = by.get(part);
    return h ? heatHex(heatRatio(h.temperature.si, map)) : toneHex.gray;
  };
}
