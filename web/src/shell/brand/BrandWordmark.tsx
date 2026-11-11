/**
 * ロゴの名前「iwafune」（輪郭にした字の SVG）。「iwa」（岩）を船底の紺、「fune」（船）を甲板の青で塗る。
 * viewBox は brand-art.ts の `wordmarkBox`（縦は x ハイトの帯の中心で上下対称）なので、
 * `BrandMark tight` と flex の items-center で並べると、マークの外枠の中心と名前の x ハイトの中心がそろう。
 */

import { brandInkHex } from "@/lib/tone";
import { WORDMARK, wordmarkBox } from "./brand-art";

/** `size` は 1 em の大きさ [px]（字の大きさ。CSS の font-size に当たる） */
export function BrandWordmark({ size }: { size: number }) {
  const box = wordmarkBox();
  const ink = brandInkHex.light;
  return (
    <svg
      width={(box.w * size) / 1000}
      height={(box.h * size) / 1000}
      viewBox={`${box.x} ${box.y} ${box.w} ${box.h}`}
      role="img"
      aria-label="iwafune"
      focusable="false"
    >
      <path d={WORDMARK.iwa} fill={ink.iwa} />
      <path d={WORDMARK.fune} fill={ink.fune} />
    </svg>
  );
}
