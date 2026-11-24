import { Color } from "three";
import { heatHex } from "@/lib/tone";

/** 配色の段数（`heatHex` を一度だけ引いて表にする。凡例と同じ配色） */
export const LUT_STEPS = 64;
const LUT: readonly (readonly [number, number, number])[] = Array.from(
  { length: LUT_STEPS + 1 },
  (_, i) => new Color(heatHex(i / LUT_STEPS)).toArray() as [number, number, number],
);

/**
 * 場の値（0 〜 1。範囲外は端）→ 色（RGB 0 〜 1）。配色は `lib/tone.ts` の `heatHex`（熱画像と同じ連続の配色）で、
 * 3D の場の色はすべてこれを通る。`heatHex` の `LUT_STEPS` 段の表（段の間は近い段）
 */
export function fieldRgb(ratio: number): readonly [number, number, number] {
  const i = Math.round(Math.min(1, Math.max(0, ratio)) * LUT_STEPS);
  return LUT[i] ?? [0, 0, 0];
}
