import type { WireEnds } from "./layout-estimate";
import { type Box, type Placement, snapDown, snapUp } from "./layout-geometry";
import type { Tree } from "./layout-pack";

/** いちばん外の枠どうしのすき間の最小 [px]。線が数本並んで通れる幅で、これより近づけない */
export const MIN_GAP = 64;
/** 寄せる繰り返しの上限（動かなくなればそこで終わる） */
const ITERATIONS = 12;

/**
 * 詰め: 格子の升目に置いたいちばん外の枠を、横・縦の順に、線の相手のほうへ寄せる（相手と揃う位置の中央値）。
 * 同じ帯（もう一方の向きで MIN_GAP 以内に重なる）にある枠とは、いまの並びの順と MIN_GAP のすき間を守るので、
 * 枠は重ならず、近づきすぎない。升目の大きさで空いていた余分なすき間と線の長さが縮む。`placement` を書き換える。
 */
export function compact(tree: Tree, placement: Placement, wires: readonly WireEnds[]): void {
  const roots = tree.roots.filter((r) => placement.absolute.has(r) && placement.boxes.has(r));
  if (roots.length < 2) return;
  const rootOf = new Map<string, string>();
  const mark = (id: string, root: string) => {
    rootOf.set(id, root);
    for (const k of tree.children.get(id) ?? []) mark(k, root);
  };
  for (const r of roots) mark(r, r);
  const box = (id: string) => placement.boxes.get(id) as Box;
  const pos = new Map(roots.map((r) => [r, { ...(placement.absolute.get(r) ?? { x: 0, y: 0 }) }]));
  // カードの中心の、いちばん外の枠の左上からの位置
  const offset = (id: string) => {
    const a = placement.absolute.get(id);
    const r = placement.absolute.get(rootOf.get(id) ?? "");
    const b = placement.boxes.get(id);
    return a && r && b
      ? { x: a.x - r.x + b.width / 2, y: a.y - r.y + b.height / 2 }
      : { x: 0, y: 0 };
  };
  const links = wires.flatMap((w) => {
    const ra = rootOf.get(w.a);
    const rb = rootOf.get(w.b);
    return ra && rb && ra !== rb ? [{ ra, rb, oa: offset(w.a), ob: offset(w.b) }] : [];
  });
  for (const axis of ["x", "y"] as const) {
    const other = axis === "x" ? "y" : "x";
    const len = axis === "x" ? "width" : "height";
    const olen = axis === "x" ? "height" : "width";
    const sameBand = (p: string, q: string) => {
      const [a, b] = [pos.get(p)?.[other] ?? 0, pos.get(q)?.[other] ?? 0];
      return a < b + box(q)[olen] + MIN_GAP && b < a + box(p)[olen] + MIN_GAP;
    };
    for (let it = 0; it < ITERATIONS; it++) {
      let moved = false;
      for (const r of roots) {
        const at = pos.get(r);
        if (!at) continue;
        const wants = links.flatMap((l) =>
          l.ra === r
            ? [(pos.get(l.rb)?.[axis] ?? 0) + l.ob[axis] - l.oa[axis]]
            : l.rb === r
              ? [(pos.get(l.ra)?.[axis] ?? 0) + l.oa[axis] - l.ob[axis]]
              : [],
        );
        if (wants.length === 0) continue;
        let lo = Number.NEGATIVE_INFINITY;
        let hi = Number.POSITIVE_INFINITY;
        for (const q of roots) {
          if (q === r || !sameBand(q, r)) continue;
          const qa = pos.get(q)?.[axis] ?? 0;
          if (qa < at[axis]) lo = Math.max(lo, snapUp(qa + box(q)[len] + MIN_GAP));
          else hi = Math.min(hi, snapDown(qa - MIN_GAP - box(r)[len]));
        }
        if (lo > hi) continue;
        const sorted = wants.toSorted((a, b) => a - b);
        const median = sorted[Math.floor((sorted.length - 1) / 2)] ?? at[axis];
        const next = Math.min(hi, Math.max(lo, snapDown(median)));
        if (next !== at[axis] && Number.isFinite(next)) {
          at[axis] = next;
          moved = true;
        }
      }
      if (!moved) break;
    }
  }
  // 左上を 0, 0 にそろえ、中のものの絶対位置も動かす
  const x0 = Math.min(...roots.map((r) => pos.get(r)?.x ?? 0));
  const y0 = Math.min(...roots.map((r) => pos.get(r)?.y ?? 0));
  for (const r of roots) {
    const from = placement.absolute.get(r);
    const to = pos.get(r);
    if (!from || !to) continue;
    const dx = to.x - x0 - from.x;
    const dy = to.y - y0 - from.y;
    for (const [id, root] of rootOf) {
      const a = placement.absolute.get(id);
      if (root === r && a) placement.absolute.set(id, { x: a.x + dx, y: a.y + dy });
    }
    box(r).x += dx;
    box(r).y += dy;
  }
}
