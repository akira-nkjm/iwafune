import type { ReactNode } from "react";
import { heatHex } from "@/lib/tone";

/** 凡例の帯（CSS の線形グラデーション。色は 3D と同じ `heatHex`） */
export function fieldGradient(): string {
  return `linear-gradient(to right, ${Array.from({ length: 8 }, (_, i) => heatHex(i / 7)).join(", ")})`;
}

/**
 * 場の凡例（低い値 → 高い値の帯）。熱画像・モードの形・板の温度の分布が同じ帯を使う。
 * 端は呼び出し側が渡す（言葉は辞書、量は API の整形を通した `Formatted`）。
 */
export function FieldLegend({
  low,
  high,
  label,
}: {
  low: ReactNode;
  high: ReactNode;
  label: string;
}) {
  return (
    <div className="flex items-center gap-2 text-ink-2 text-xs" role="img" aria-label={label}>
      <span>{low}</span>
      <span className="h-3 w-48 max-w-full rounded-sm" style={{ background: fieldGradient() }} />
      <span>{high}</span>
    </div>
  );
}
