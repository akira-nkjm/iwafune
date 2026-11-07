import type { Pt } from "./layout-geometry";

/** 折れ線の線分 */
export interface Seg {
  a: Pt;
  b: Pt;
}

export function segmentsOf(points: readonly Pt[]): Seg[] {
  const out: Seg[] = [];
  for (let i = 1; i < points.length; i++) out.push({ a: points[i - 1] as Pt, b: points[i] as Pt });
  return out;
}

const horizontal = (s: Seg) => Math.abs(s.a.y - s.b.y) < 0.5;
const vertical = (s: Seg) => Math.abs(s.a.x - s.b.x) < 0.5;
const between = (v: number, p: number, q: number, pad: number) =>
  v > Math.min(p, q) + pad && v < Math.max(p, q) - pad;

/** 水平の線分 h が、ほかの辺の垂直の線分と交わる x（線の飛び越しを描く位置。重なる交差は 1 つ）。描画のための幾何で、物理量ではない */
export function crossingsOn(h: Seg, others: readonly Seg[], radius: number): number[] {
  if (!horizontal(h)) return [];
  const xs: number[] = [];
  for (const v of others) {
    if (!vertical(v)) continue;
    if (between(v.a.x, h.a.x, h.b.x, radius) && between(h.a.y, v.a.y, v.b.y, 0.5)) xs.push(v.a.x);
  }
  // 同じ所を通る複数の線（幹を共有する線など）や、飛び越しの幅より近い交差は 1 つにまとめる
  // （同じ位置に何度も飛び越しを描くと、線が行き来してぎざぎざになる）
  const sorted = xs.sort((p, q) => (h.a.x <= h.b.x ? p - q : q - p));
  return sorted.filter((x, i) => i === 0 || Math.abs(x - (sorted[i - 1] ?? x)) >= 2 * radius);
}

/**
 * 折れ線を SVG の path にする。水平の線分がほかの辺（ほかのネットの線。line-store.ts）の垂直の線分をまたぐところには、
 * 半円の「飛び越し」を入れる（交差と接続を見分けるため）。
 */
export function jumpPath(points: readonly Pt[], others: readonly Seg[], radius = 4): string {
  const first = points[0];
  if (!first) return "";
  let d = `M ${first.x} ${first.y}`;
  for (const s of segmentsOf(points)) {
    const dir = s.b.x >= s.a.x ? 1 : -1;
    for (const x of crossingsOn(s, others, radius)) {
      d += ` L ${x - dir * radius} ${s.a.y} A ${radius} ${radius} 0 0 ${dir > 0 ? 1 : 0} ${x + dir * radius} ${s.a.y}`;
    }
    d += ` L ${s.b.x} ${s.b.y}`;
  }
  return d;
}

/** 折れ線の一番長い線分の中点（ラベルの位置） */
export function labelPoint(points: readonly Pt[]): Pt {
  let best: Seg | null = null;
  let len = -1;
  for (const s of segmentsOf(points)) {
    const l = Math.abs(s.b.x - s.a.x) + Math.abs(s.b.y - s.a.y);
    if (l > len) {
      len = l;
      best = s;
    }
  }
  if (!best) return points[0] ?? { x: 0, y: 0 };
  return { x: (best.a.x + best.b.x) / 2, y: (best.a.y + best.b.y) / 2 };
}
