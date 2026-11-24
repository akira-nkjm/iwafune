import { type Box, GRID, type Pt, type Side, snapDown, snapUp } from "./geometry";
import { Board, type Frame, Work } from "./route-board";

export type { Frame } from "./route-board";

import { tidyRoute } from "./routes";

/**
 * 線の経路探索（直交、GRID の格子の上）。カードと枠の見出しを避け、ほかのネットの線とは重ねず（平行に重なる所は
 * ほぼ通れない費用）、なるべく離し、交差と折れを減らす。同じネットの線は同じ所を通ってよい（幹を共有して分岐する）。
 * 全部の線を 1 本ずつ引いたあと、1 本ずつ外して引き直す（後から引いた線に合わせて先の線も譲る）。
 */

/** 1 格子進む費用を 1 としたときの、各条件の費用 */
const COST = {
  /** 折れ 1 回 */
  bend: 10,
  /** ほかのネットの線との交差 1 つ */
  cross: 30,
  /** ほかのネットの線と平行に重なる 1 格子（実際には通らない） */
  overlap: 400,
  /** ほかのネットの線のすぐ隣（1 格子）を平行に走る 1 格子 */
  near: 1.5,
  /** ほかのネットの線の上で折れる（つながって見える） */
  corner: 40,
  /** 線の両端が入っていない枠の中を通る 1 格子（枠の間の通り道を優先する） */
  foreign: 0.4,
  /** 枠の縁（とその内外 1 格子）に沿って走る 1 格子 */
  border: 3,
  /** 同じネットの線の上を進む 1 格子（幹の共有を少しだけ好む） */
  share: 0.9,
} as const;
/** 探す範囲: 両端を囲む長方形をこれだけの格子だけ広げる（見つからなければ全体で探す） */
const MARGIN = 30;

/** 線の端。`side` は端子の向き（その向きへ出る）。null はジャンクション（どの向きからでもよい） */
export interface Terminal {
  at: Pt;
  side: Side | null;
}

export interface Wire {
  id: string;
  /** ネット（同じなら重ねてよい） */
  net: string;
  from: Terminal;
  to: Terminal;
  /** 線が自由に通ってよい枠（両端の祖先の枠と組立品） */
  frames: ReadonlySet<string>;
}

const DX = [1, -1, 0, 0] as const;
const DY = [0, 0, 1, -1] as const;
const OPPOSITE = [1, 0, 3, 2] as const;
/** 向き → 線の向き（0: 横、1: 縦） */
const AXIS = [0, 0, 1, 1] as const;
const OUTWARD: Record<Side, number> = { right: 0, left: 1, bottom: 2, top: 3 };

/** 端子から格子へ出る最初の点（カードの縁から 1 格子以上離れた所）。ジャンクションはその点そのもの */
function stubOf(t: Terminal): Pt {
  const { x, y } = t.at;
  switch (t.side) {
    case "right":
      return { x: snapUp(x + GRID - 3), y };
    case "left":
      return { x: snapDown(x - GRID + 3), y };
    case "bottom":
      return { x, y: snapUp(y + GRID - 3) };
    case "top":
      return { x, y: snapDown(y - GRID + 3) };
    default:
      return { x: snapUp(x - GRID / 2), y: snapUp(y - GRID / 2) };
  }
}

interface Search {
  start: number;
  startDir: number;
  goal: number;
  goalDir: number;
  net: number;
  /** 枠の番号 + 1（盤面の frame）→ 線の両端と関係の無い枠なら 1（0 番は枠の外で 0） */
  foreign: Uint8Array;
  box: [number, number, number, number];
}

/**
 * 1 本の線の経路（格子点の番号の列）とその費用。A*（状態は格子点 × 進む向き）。
 * 内側の繰り返しなので、費用の足し方（順序も含めて）は変えずに、呼び出しと配列の範囲外の読みを避けて書いている。
 */
function search(board: Board, work: Work, s: Search): { path: number[]; cost: number } | null {
  const { nx, ny, blocked, border, frame } = board;
  const states = nx * ny * 4;
  const end = states;
  work.ensure(states + 1);
  work.stamp += 1;
  const { g, parent, seen, closed, stamp, heap } = work;
  heap.clear();
  const [bx0, by0, bx1, by1] = s.box;
  const gxGoal = s.goal % nx;
  const gyGoal = Math.floor(s.goal / nx);
  const net1 = s.net + 1;
  const [occH, occV] = board.occ;
  const relax = (state: number, cost: number, from: number, h: number) => {
    if (seen[state] === stamp && (g[state] ?? 0) <= cost) return;
    seen[state] = stamp;
    g[state] = cost;
    parent[state] = from;
    heap.push(cost + h, state);
  };
  // 残りの見積もり（格子点の座標から。ゴールまでの格子の数 × 共有する線の上の費用）
  const rest = (x: number, y: number) => COST.share * (Math.abs(x - gxGoal) + Math.abs(y - gyGoal));
  const [sx, sy] = [s.start % nx, Math.floor(s.start / nx)];
  for (let d = 0; d < 4; d++)
    if (s.startDir < 0 || d === s.startDir) relax(s.start * 4 + d, 0, -1, rest(sx, sy));
  while (heap.size > 0) {
    const state = heap.pop();
    if (state === end) return { path: trace(parent, state), cost: g[state] ?? 0 };
    if (closed[state] === stamp) continue;
    closed[state] = stamp;
    const i = state >> 2;
    const d = state & 3;
    const base = g[state] ?? 0;
    if (i === s.goal) {
      relax(end, base + (s.goalDir < 0 || d === s.goalDir ? 0 : COST.bend), state, 0);
      continue;
    }
    const gx = i % nx;
    const gy = Math.floor(i / nx);
    // 今いる点が別ネットの線の上か（そこで折れると、つながって見える）
    const oh = occH[i] ?? 0;
    const ov = occV[i] ?? 0;
    const onOther = (oh !== 0 && oh !== net1) || (ov !== 0 && ov !== net1);
    const ib = border[i] ?? 0;
    for (let nd = 0; nd < 4; nd++) {
      if (nd === OPPOSITE[d]) continue;
      const qx = gx + (DX[nd] ?? 0);
      const qy = gy + (DY[nd] ?? 0);
      if (qx < bx0 || qy < by0 || qx > bx1 || qy > by1) continue;
      const q = qy * nx + qx;
      if (blocked[q] && q !== s.goal) continue;
      // 1 格子進む費用。足す順は変えない（浮動小数点の丸めが変わると、同じ費用の経路の選び方が変わりうる）
      const horizontal = nd < 2;
      const along = horizontal ? occH : occV;
      const across = horizontal ? occV : occH;
      const here = along[q] ?? 0;
      let c = here === net1 ? COST.share : 1;
      if (nd !== d) {
        c += COST.bend;
        if (onOther) c += COST.corner;
      }
      if (here !== 0 && here !== net1) c += COST.overlap;
      const cross = across[q] ?? 0;
      if (cross !== 0 && cross !== net1) c += COST.cross;
      const step = horizontal ? nx : 1;
      const n1 = q - step >= 0 ? (along[q - step] ?? 0) : 0;
      const n2 = q + step < along.length ? (along[q + step] ?? 0) : 0;
      if ((n1 !== 0 && n1 !== net1) || (n2 !== 0 && n2 !== net1)) c += COST.near;
      const bit = horizontal ? 1 : 2;
      if (((border[q] ?? 0) & bit) !== 0 && (ib & bit) !== 0) c += COST.border;
      if (s.foreign[frame[q] ?? 0]) c += COST.foreign;
      relax(q * 4 + nd, base + c, state, rest(qx, qy));
    }
  }
  return null;
}

function trace(parent: Int32Array, end: number): number[] {
  const out: number[] = [];
  for (let s = parent[end] ?? -1; s >= 0; s = parent[s] ?? -1) out.push(s >> 2);
  return out.reverse();
}

/** 引く線 1 本の段取り: 線、両端の格子点、ネットの番号 */
interface Plan {
  w: Wire;
  a: number;
  b: number;
  net: number;
}

/** 経路が見つからなかった線の費用（どの候補よりも大きい） */
const UNROUTED = 1e6;

/**
 * 全部の線を引く係。線を 1 本ずつ引いたあと、1 本ずつ外して引き直す。引き終えたあと、1 本の線の端を別の所に
 * 付け替えたらどれだけの費用で引けるか（ほかの線はそのまま）も比べられる（端子の側の選び直し。layout.ts）。
 */
export class Router {
  private readonly board: Board;
  private readonly work = new Work();
  private readonly plan: Plan[];
  private readonly frameIds: string[];
  private readonly paths = new Map<string, number[]>();
  private readonly costs = new Map<string, number>();

  constructor(wires: Wire[], cards: Box[], frames: Frame[]) {
    this.board = new Board(
      cards,
      frames,
      wires.flatMap((w) => [w.from.at, w.to.at]),
    );
    const nets = [...new Set(wires.map((w) => w.net))];
    const netOf = new Map(nets.map((n, i) => [n, i]));
    this.frameIds = frames.toSorted((a, b) => a.depth - b.depth).map((f) => f.id);
    this.plan = wires.map((w) => this.planOf(w, netOf.get(w.net) ?? 0));
    // ネットの線の多い順、同じネットの中は遠い線から（幹を先に引く）
    const size = (net: string) => wires.filter((w) => w.net === net).length;
    const dist = (p: Plan) =>
      Math.abs(p.w.from.at.x - p.w.to.at.x) + Math.abs(p.w.from.at.y - p.w.to.at.y);
    this.plan.sort(
      (p, q) =>
        size(q.w.net) - size(p.w.net) || p.w.net.localeCompare(q.w.net) || dist(q) - dist(p),
    );
  }

  private planOf(w: Wire, net: number): Plan {
    const index = (p: Pt) => {
      const [gx, gy] = this.board.cell(p);
      return gy * this.board.nx + gx;
    };
    return { w, a: index(stubOf(w.from)), b: index(stubOf(w.to)), net };
  }

  /** 盤面に、`skip` 以外の線と、全部の端子の出口（端子から格子までの短い線。その端子のネットのもの）を置く */
  private occupy(skip: string | null) {
    this.board.clear();
    for (const p of this.plan) {
      if (p.w.from.side) this.board.mark(AXIS[OUTWARD[p.w.from.side]] ?? 0, p.a, p.net);
      if (p.w.to.side) this.board.mark(AXIS[OUTWARD[p.w.to.side]] ?? 0, p.b, p.net);
      const path = p.w.id === skip ? null : this.paths.get(p.w.id);
      if (path) markPath(this.board, path, p.net);
    }
  }

  /** 全部の線を引く（2 回目は 1 本ずつ外して引き直す） */
  routeAll() {
    for (const pass of [0, 1])
      for (const p of this.plan) {
        this.occupy(pass === 1 ? p.w.id : null);
        const r = routeOne(this.board, this.work, p, this.frameIds);
        this.paths.set(p.w.id, r.path);
        this.costs.set(p.w.id, r.cost);
      }
  }

  /** 線 `id` の端（`end`）を `terminal` に付け替えたときの、その線だけを引き直した費用（ほかの線はそのまま） */
  costWith(id: string, end: "from" | "to", terminal: Terminal): number {
    const p = this.plan.find((x) => x.w.id === id);
    if (!p) return UNROUTED;
    this.occupy(id);
    return routeOne(
      this.board,
      this.work,
      this.planOf({ ...p.w, [end]: terminal }, p.net),
      this.frameIds,
    ).cost;
  }

  /** 引いた線の費用（routeAll の 2 回目のもの） */
  cost(id: string): number {
    return this.costs.get(id) ?? UNROUTED;
  }

  result(): Map<string, Pt[]> {
    const out = new Map<string, Pt[]>();
    for (const p of this.plan) {
      const path = this.paths.get(p.w.id) ?? [];
      out.set(p.w.id, tidyRoute([p.w.from.at, ...path.map((i) => this.board.point(i)), p.w.to.at]));
    }
    return out;
  }
}

/**
 * 線をすべて引く。戻り値は線ごとの経路（絶対座標の折れ線。両端は端子の点）。`unrouted` を渡すと、経路が見つからず
 * 端どうしを段差でつないだだけの線の ID を入れる（候補を比べるときの罰点。score.ts）。
 */
export function routeWires(
  wires: Wire[],
  cards: Box[],
  frames: Frame[],
  unrouted?: Set<string>,
): Map<string, Pt[]> {
  return routeKeeping(wires, cards, frames, unrouted).routes;
}

/**
 * `routeWires` と同じ引き方で、引き終えた係（`Router`）も返す。同じ配置の線の端を選び直すとき（`compareEnds`）に
 * 渡せば、全部の線を引き直さずに済む（同じ入力なら同じ経路なので、結果は変わらない）。
 */
export function routeKeeping(
  wires: Wire[],
  cards: Box[],
  frames: Frame[],
  unrouted?: Set<string>,
): { routes: Map<string, Pt[]>; router: Router } {
  const router = new Router(wires, cards, frames);
  router.routeAll();
  for (const w of wires) if (router.cost(w.id) >= UNROUTED) unrouted?.add(w.id);
  return { routes: router.result(), router };
}

/** 線の端の付け替えの候補（線の ID・どちらの端か・付け替える先） */
export interface Alternative {
  wire: string;
  end: "from" | "to";
  terminal: Terminal;
}

/**
 * 全部の線を引いたうえで、線ごとの費用と、候補ごとに線の端を付け替えたときのその線の費用を返す
 * （ほかの線はそのまま）。`routed` に、同じ線・カード・枠を引き終えた係（`routeKeeping`）を渡すと、引き直さずにそれを使う。
 * 候補は引いた経路を見て作る（`propose`。端子の側や位置の選び直し。refine.ts）。
 * 候補の付け替え先は、カードの縁の上（盤面の中）に置くこと。
 */
export function compareEnds(
  wires: Wire[],
  cards: Box[],
  frames: Frame[],
  propose: (routes: Map<string, Pt[]>) => Alternative[],
  routed?: Router,
): { base: Map<string, number>; alternatives: Alternative[]; costs: number[] } {
  let router = routed;
  if (!router) {
    router = new Router(wires, cards, frames);
    router.routeAll();
  }
  const alternatives = propose(router.result());
  return {
    base: new Map(wires.map((w) => [w.id, router.cost(w.id)])),
    alternatives,
    costs: alternatives.map((alt) => router.costWith(alt.wire, alt.end, alt.terminal)),
  };
}

function markPath(board: Board, path: number[], net: number) {
  for (let k = 1; k < path.length; k++) {
    const a = path[k - 1] ?? 0;
    const b = path[k] ?? 0;
    const axis = Math.abs(a - b) === 1 ? 0 : 1;
    board.mark(axis, a, net);
    board.mark(axis, b, net);
  }
}

/** 枠の番号 + 1 ごとの「この線の両端と関係の無い枠か」（0 は枠の外） */
function foreignFlags(w: Wire, frameIds: string[]): Uint8Array {
  const out = new Uint8Array(frameIds.length + 1);
  frameIds.forEach((id, k) => {
    out[k + 1] = w.frames.has(id) ? 0 : 1;
  });
  return out;
}

function routeOne(
  board: Board,
  work: Work,
  p: Plan,
  frameIds: string[],
): { path: number[]; cost: number } {
  const { w, a, b } = p;
  const s: Search = {
    start: a,
    startDir: w.from.side ? OUTWARD[w.from.side] : -1,
    goal: b,
    goalDir: w.to.side ? (OPPOSITE[OUTWARD[w.to.side]] ?? -1) : -1,
    net: p.net,
    foreign: foreignFlags(w, frameIds),
    box: [0, 0, board.nx - 1, board.ny - 1],
  };
  const [ax, ay] = [a % board.nx, Math.floor(a / board.nx)];
  const [bx, by] = [b % board.nx, Math.floor(b / board.nx)];
  const near = search(board, work, {
    ...s,
    box: [
      Math.max(0, Math.min(ax, bx) - MARGIN),
      Math.max(0, Math.min(ay, by) - MARGIN),
      Math.min(board.nx - 1, Math.max(ax, bx) + MARGIN),
      Math.min(board.ny - 1, Math.max(ay, by) + MARGIN),
    ],
  });
  // 見つからなければ全体で、それでも無ければ（囲まれているとき）端どうしを段差でつなぐ
  return near ?? search(board, work, s) ?? { path: [a, b], cost: UNROUTED };
}
