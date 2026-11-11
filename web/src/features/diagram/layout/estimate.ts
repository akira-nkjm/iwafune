import type { DiagramModel } from "../model";
import { aspectMiss, type Box, type Placement, type Pt, type Size } from "./geometry";
import { besideOrBelow } from "./grid";

/**
 * 並べ方の見積もり（place.ts の探索が比べる値）: 線の長さ・交差の見込み・ほかのカードを横切る見込み・
 * 全体と枠の形。線を引く前の粗い値で、最後は線を引いてから score.ts で比べる。
 */

/** 見積もりで、枠の形のずれ（枠ごとの縦横比のずれの平均）に掛ける重み */
const FRAME_SHAPE = 0.8;
/** 見積もりで、交差の見込み 1 つを線の長さに換算した値 [px] */
const CROSSING_PX = 300;
/** 見積もりで、ほかのカードを横切る線 1 本を線の長さに換算した値 [px] */
const BLOCKED_PX = 200;

/** 線の両端のカードとネット（ジャンクションの線は、そのジャンクションにつながる最初のカードとの間とみなす） */
export type WireEnds = { a: string; b: string; net: string };

export function wireEnds(model: DiagramModel): WireEnds[] {
  const first = new Map<string, string>();
  const junctions = new Set(model.junctions.map((j) => j.id));
  const card = (id: string, other: string) => {
    if (!junctions.has(id)) return id;
    const f = first.get(id);
    if (f === undefined) first.set(id, other);
    return f ?? other;
  };
  return model.edges.map((e) => ({
    a: card(e.source.node, e.target.node),
    b: card(e.target.node, e.source.node),
    net: e.nets.toSorted().join("+") || e.id,
  }));
}

/** 2 つの線分が端以外の所で交わるか（向きの符号で判定） */
function intersects(p: Pt, q: Pt, r: Pt, s: Pt): boolean {
  const turn = (a: Pt, b: Pt, c: Pt) =>
    Math.sign((b.x - a.x) * (c.y - a.y) - (b.y - a.y) * (c.x - a.x));
  return turn(p, q, r) * turn(p, q, s) < 0 && turn(r, s, p) * turn(r, s, q) < 0;
}

/**
 * 交差の見込み: 線の両端のカードの中心を直線で結んだとき、ほかのネットの線と交わる組の数。線を引く前に、交差の
 * 少ない置き方（線の相手どうしが向かい合う置き方）を選ぶための値。同じカードを端に持つ線どうしは数えない。
 */
function crossingsOf(lines: { a: string; b: string; net: string; p: Pt; q: Pt }[]): number {
  let n = 0;
  for (const [i, l] of lines.entries())
    for (const m of lines.slice(i + 1))
      if (
        l.net !== m.net &&
        l.a !== m.a &&
        l.a !== m.b &&
        l.b !== m.a &&
        l.b !== m.b &&
        intersects(l.p, l.q, m.p, m.q)
      )
        n++;
  return n + interleaved(lines);
}

/**
 * 同じカードから出る別々のネットの線の、相手の並びの入り組み方。カードの周りを一周したときに相手のネットが入れ替わる
 * 回数（同じネットのひと続きの数）から、ネットの数を引いたもの。電源の配電のように 1 枚のカードから何本もネットが
 * 出るとき、同じネットの相手がまとまっていないと線は必ず交わる（直線で結ぶ見込みでは、端を共有するので数えられない）。
 */
function interleaved(lines: { a: string; b: string; net: string; p: Pt; q: Pt }[]): number {
  const around = new Map<string, { angle: number; net: string }[]>();
  for (const l of lines)
    for (const [hub, at, other] of [
      [l.a, l.p, l.q],
      [l.b, l.q, l.p],
    ] as const)
      around.set(hub, [
        ...(around.get(hub) ?? []),
        { angle: Math.atan2(other.y - at.y, other.x - at.x), net: l.net },
      ]);
  let n = 0;
  for (const list of around.values()) {
    const nets = new Set(list.map((x) => x.net));
    if (nets.size < 2) continue;
    const order = list.toSorted((x, y) => x.angle - y.angle).map((x) => x.net);
    const runs = order.filter(
      (net, i) => net !== order[(i + order.length - 1) % order.length],
    ).length;
    n += Math.max(0, runs - nets.size);
  }
  return n;
}

/**
 * 並べ方の見積もり: 線の両端のカードの中心の間の距離（縦 + 横）の合計と CROSSING_PX × 交差の見込みの和に、「1 + 全体（線の無い枠を下か右に置いたあと）の
 * 縦横比のずれ + FRAME_SHAPE × 枠（中に 2 つ以上あるもの）の縦横比のずれの平均」を掛けたもの。線を引く前の、候補を絞るための粗い値（最後は線を引いてから score.ts で比べる）。
 */
export function estimate(
  placement: Placement,
  ends: WireEnds[],
  unlinked: readonly Size[],
  frames: readonly string[],
  cards: readonly string[],
): number {
  const center = (id: string) => {
    const p = placement.absolute.get(id);
    const b = placement.boxes.get(id);
    return p && b ? { x: p.x + b.width / 2, y: p.y + b.height / 2 } : null;
  };
  let length = 0;
  const lines: { a: string; b: string; net: string; p: Pt; q: Pt }[] = [];
  for (const e of ends) {
    const p = center(e.a);
    const q = center(e.b);
    if (!p || !q) continue;
    length += Math.abs(p.x - q.x) + Math.abs(p.y - q.y);
    lines.push({ ...e, p, q });
  }
  let x1 = 0;
  let y1 = 0;
  for (const [id, p] of placement.absolute) {
    const b = placement.boxes.get(id);
    if (b) {
      x1 = Math.max(x1, p.x + b.width);
      y1 = Math.max(y1, p.y + b.height);
    }
  }
  const whole = besideOrBelow({ width: x1, height: y1 }, unlinked);
  // 枠の形: カードを 1 列に積んだ縦長の枠や 1 行に並べた横長の枠は、図全体が列に見えるので嫌う
  const misses = frames.flatMap((id) => {
    const b = placement.boxes.get(id);
    return b ? [aspectMiss(b.width, b.height)] : [];
  });
  const frameMiss = misses.length > 0 ? misses.reduce((a, b) => a + b, 0) / misses.length : 0;
  const boxes = cards.flatMap((id) => {
    const p = placement.absolute.get(id);
    const b = placement.boxes.get(id);
    return p && b ? [{ id, x: p.x, y: p.y, width: b.width, height: b.height }] : [];
  });
  return (
    (length + CROSSING_PX * crossingsOf(lines) + BLOCKED_PX * blockedOf(lines, boxes)) *
    (1 + aspectMiss(whole.width, whole.height) + FRAME_SHAPE * frameMiss)
  );
}

/**
 * ほかのカードを横切る見込み: 線の両端のカードの中心を縦横の 2 区間（横に出てから縦、縦に出てから横）で結んだとき、
 * どちらの結び方でも両端以外のカードの上を通る線の数。直交の線は、そうなるとカードを回り込むしかない。横に並べた
 * カードの奥のカードから出る線が手前のカードを回り込む、といった並べ方を嫌う（縦に積めばまっすぐ出られる）。
 */
function blockedOf(
  lines: { a: string; b: string; p: Pt; q: Pt }[],
  boxes: (Box & { id: string })[],
): number {
  let n = 0;
  for (const l of lines) {
    const others = boxes.filter((b) => b.id !== l.a && b.id !== l.b);
    const hits = (u: Pt, v: Pt) => others.some((b) => segmentHits(u, v, b));
    const corner1 = { x: l.q.x, y: l.p.y };
    const corner2 = { x: l.p.x, y: l.q.y };
    const viaH = hits(l.p, corner1) || hits(corner1, l.q);
    const viaV = hits(l.p, corner2) || hits(corner2, l.q);
    if (viaH && viaV) n++;
  }
  return n;
}

/** 縦か横の線分が箱の内側（縁から 2 px 内）を通るか */
function segmentHits(u: Pt, v: Pt, b: Box): boolean {
  const [x0, x1, y0, y1] = [b.x + 2, b.x + b.width - 2, b.y + 2, b.y + b.height - 2];
  if (u.y === v.y)
    return u.y > y0 && u.y < y1 && Math.max(u.x, v.x) > x0 && Math.min(u.x, v.x) < x1;
  return u.x > x0 && u.x < x1 && Math.max(u.y, v.y) > y0 && Math.min(u.y, v.y) < y1;
}
