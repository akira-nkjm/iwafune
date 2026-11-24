import { heatHex } from "./tone";

/** 色の範囲（SI の K）。熱画像（`features/systems/thermal`）と、メッシュ上の温度の場（`analysis/detail`）が同じ意味で使う */
export interface HeatScale {
  min: number;
  max: number;
}

/** 値の並びの最低〜最高。値が 1 つも無ければ null（NaN は数えない） */
export function heatScaleOfValues(values: Iterable<number>): HeatScale | null {
  let min = Number.POSITIVE_INFINITY;
  let max = Number.NEGATIVE_INFINITY;
  for (const v of values) {
    if (Number.isNaN(v)) continue;
    min = Math.min(min, v);
    max = Math.max(max, v);
  }
  return Number.isFinite(min) ? { min, max } : null;
}

/**
 * 色の位置（0 = 範囲の最低温度、1 = 最高温度）。色を塗るためだけの比で、物理量ではない。色そのものは
 * `lib/tone.ts` の `heatHex`（3D と凡例で共通）。範囲の幅が 0 なら真ん中。
 */
export function heatRatio(t: number, scale: HeatScale): number {
  return scale.max > scale.min ? (t - scale.min) / (scale.max - scale.min) : 0.5;
}

/** 温度 `t` の色（hex）。熱画像の部品も、メッシュ上の場の凡例の見本も、この 1 つの写し方 */
export function heatColor(t: number, scale: HeatScale): string {
  return heatHex(heatRatio(t, scale));
}
