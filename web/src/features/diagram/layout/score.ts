import type { DiagramModel } from "../model";
import { aspectMiss, type DiagramLayout, extentOf, type Pt, type Size } from "./geometry";
import { besideOrBelow } from "./grid";

/** 線の混み具合（自動配置の候補を比べるための値） */
export interface LineStats {
  /** 線どうしの交差の数（同じ端子から出る線・同じネットの線どうしは、つながっているので数えない） */
  crossings: number;
  /** 線どうしが同じ所を平行に重なって走る区間の数（同じ端子・同じネットの線どうしは幹の共有なので数えない） */
  overlaps: number;
  /** 折れの数の合計 */
  bends: number;
  /** 線の長さの合計 [px] */
  length: number;
}

type Segment = { a: Pt; b: Pt; edge: string };

/** 水平の線分と垂直の線分が、どちらの端でもない所で交わるか */
function crosses(h: Segment, v: Segment): boolean {
  const [x0, x1] = [Math.min(h.a.x, h.b.x), Math.max(h.a.x, h.b.x)];
  const [y0, y1] = [Math.min(v.a.y, v.b.y), Math.max(v.a.y, v.b.y)];
  return v.a.x > x0 && v.a.x < x1 && h.a.y > y0 && h.a.y < y1;
}

/** 自動配置の結果の、線の交差・折れ・長さ */
export function lineStats(model: DiagramModel, layout: DiagramLayout): LineStats {
  const ends = new Map(
    model.edges.map((e) => [
      e.id,
      [e.source.port ?? e.source.node, e.target.port ?? e.target.node],
    ]),
  );
  const nets = new Map(model.edges.map((e) => [e.id, e.nets]));
  const shareEnd = (a: string, b: string) => {
    const [as, at] = ends.get(a) ?? [];
    const [bs, bt] = ends.get(b) ?? [];
    const na = nets.get(a) ?? [];
    return as === bs || at === bt || (nets.get(b) ?? []).some((n) => na.includes(n));
  };
  const horizontal: Segment[] = [];
  const vertical: Segment[] = [];
  let bends = 0;
  let length = 0;
  for (const [edge, pts] of layout.routes) {
    bends += Math.max(0, pts.length - 2);
    pts.slice(1).forEach((b, i) => {
      const a = pts[i] as Pt;
      length += Math.abs(b.x - a.x) + Math.abs(b.y - a.y);
      if (a.y === b.y && a.x !== b.x) horizontal.push({ a, b, edge });
      else if (a.x === b.x && a.y !== b.y) vertical.push({ a, b, edge });
    });
  }
  let crossings = 0;
  for (const h of horizontal)
    for (const v of vertical)
      if (h.edge !== v.edge && !shareEnd(h.edge, v.edge) && crosses(h, v)) crossings++;
  let overlaps = 0;
  for (const segs of [horizontal, vertical])
    for (const [i, a] of segs.entries())
      for (const b of segs.slice(i + 1))
        if (a.edge !== b.edge && !shareEnd(a.edge, b.edge) && collinear(a, b)) overlaps++;
  return { crossings, overlaps, bends, length };
}

/** 平行な 2 つの線分が同じ直線の上で 1 px より長く重なるか */
function collinear(a: Segment, b: Segment): boolean {
  const horizontal = a.a.y === a.b.y;
  if (horizontal ? a.a.y !== b.a.y : a.a.x !== b.a.x) return false;
  const lo = (s: Segment) => (horizontal ? Math.min(s.a.x, s.b.x) : Math.min(s.a.y, s.b.y));
  const hi = (s: Segment) => (horizontal ? Math.max(s.a.x, s.b.x) : Math.max(s.a.y, s.b.y));
  return Math.min(hi(a), hi(b)) - Math.max(lo(a), lo(b)) > 1;
}

/**
 * 自動配置の候補を比べる重み。重なり・縦横比のずれ（比の対数の差）・交差・折れ・線の総延長（図の対角線で割る）・
 * 経路が見つからなかった線を足し合わせる。重なりは線が読めなくなるので重く、交差は少ないほうがよいが、折れと長さほど
 * 重くは見ない（交差には飛び越しを描くので読める）。経路の見つからない線は、どの候補よりも悪くする。
 */
export const COST = {
  crossing: 0.5,
  overlap: 4,
  aspect: 8,
  bend: 0.4,
  length: 1,
  unrouted: 20,
} as const;

/** 候補の費用（小さいほど読みやすい）。線の無い枠（`unlinked`）を下か右に置いたあとの全体の形で比べる */
export function layoutCost(
  model: DiagramModel,
  layout: DiagramLayout,
  unlinked: readonly Size[],
): number {
  const whole = besideOrBelow(extentOf([...layout.absolute.keys()], layout), unlinked);
  const s = lineStats(model, layout);
  return (
    COST.unrouted * (layout.unrouted?.size ?? 0) +
    COST.crossing * s.crossings +
    COST.overlap * s.overlaps +
    COST.aspect * aspectMiss(whole.width, whole.height) +
    COST.bend * s.bends +
    (COST.length * s.length) / Math.hypot(whole.width, whole.height)
  );
}
