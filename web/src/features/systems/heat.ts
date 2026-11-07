import type { ThermalSteadyOutput, Verdict } from "@/api/types";
import { toneHex } from "@/lib/tone";

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
  /** 範囲の端から何 K 以内を注意にするか（出力の range_warning_band。温度差 [K]） */
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
  const band = output.range_warning_band;
  return { cases, nodes, cells, min, max, band };
}

/** 色の位置（0 = 表の最低温度、1 = 最高温度）。色を塗るためだけの比で、物理量ではない */
export function heatRatio(t: number, map: Pick<HeatMap, "min" | "max">): number {
  return map.max > map.min ? (t - map.min) / (map.max - map.min) : 0.5;
}

/** 寒色（accent）→ 暖色（neg）の塗り。color-mix で混ぜる */
export function heatColor(ratio: number): string {
  const p = Math.round(Math.max(0, Math.min(1, ratio)) * 100);
  return `color-mix(in oklab, var(--color-neg) ${p}%, var(--color-accent))`;
}

/** 3D（three.js は color-mix を読めない）用の同じ塗り。accent（blue）→ neg（red）を sRGB で線形に混ぜた hex */
export function heatHex(ratio: number): string {
  const rgb = (hex: string) => [1, 3, 5].map((i) => Number.parseInt(hex.slice(i, i + 2), 16));
  const a = rgb(toneHex.blue);
  const b = rgb(toneHex.red);
  const r = Math.max(0, Math.min(1, ratio));
  return `#${a
    .map((x, i) =>
      Math.round(x + ((b[i] ?? x) - x) * r)
        .toString(16)
        .padStart(2, "0"),
    )
    .join("")}`;
}
