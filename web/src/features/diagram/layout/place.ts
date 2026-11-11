import type { DiagramModel } from "../model";
import { estimate, wireEnds } from "./estimate";
import { type Placement, type Pt, type Size, snapDown } from "./geometry";
import { defaultColumns, type Packing, pack, type Tree } from "./pack";

/** いちばん外の枠どうしのすき間 [px]。枠の間を線がまとまって通る */
const ROOT_GAP = 80;
/** 局所探索の繰り返しの上限（改善が止まればそこで終わる） */
const ROUNDS = 8;

/** 並べ方: 格子の形（行と列）、いちばん外のもの → 格子の升目の番号、中の並べ方 */
export interface Arrangement {
  rows: number;
  cols: number;
  slot: Map<string, number>;
  packing: Packing;
}

/**
 * いちばん外の枠とカードを格子の升目に、中のものは枠ごとの格子に置く。升目の列の幅と行の高さはそこに置いた枠の
 * 大きさの最大で、枠は升目の中ほどに置く。空の列と行は詰める。
 */
export function place(tree: Tree, arr: Arrangement, sizeOf: (id: string) => Size): Placement {
  const boxes = pack(tree, arr.packing, sizeOf);
  const colW = Array.from({ length: arr.cols }, () => 0);
  const rowH = Array.from({ length: arr.rows }, () => 0);
  for (const [id, s] of arr.slot) {
    const b = boxes.get(id);
    if (!b) continue;
    const c = s % arr.cols;
    const r = Math.floor(s / arr.cols);
    colW[c] = Math.max(colW[c] ?? 0, b.width);
    rowH[r] = Math.max(rowH[r] ?? 0, b.height);
  }
  const starts = (sizes: number[]) => {
    const out: number[] = [];
    let at = 0;
    for (const s of sizes) {
      out.push(at);
      if (s > 0) at += s + ROOT_GAP;
    }
    return out;
  };
  const xs = starts(colW);
  const ys = starts(rowH);
  const absolute = new Map<string, Pt>();
  const walk = (id: string, x: number, y: number) => {
    absolute.set(id, { x, y });
    for (const k of arr.packing.order.get(id) ?? tree.children.get(id) ?? []) {
      const b = boxes.get(k);
      if (b) walk(k, x + b.x, y + b.y);
    }
  };
  for (const [id, s] of arr.slot) {
    const b = boxes.get(id);
    if (!b) continue;
    const c = s % arr.cols;
    const r = Math.floor(s / arr.cols);
    b.x = (xs[c] ?? 0) + snapDown(((colW[c] ?? 0) - b.width) / 2);
    b.y = (ys[r] ?? 0) + snapDown(((rowH[r] ?? 0) - b.height) / 2);
    walk(id, b.x, b.y);
  }
  return { boxes, absolute };
}

/** 格子の形の候補（行 × 列）。升目が足りて、行がまるごと空かない形 */
function shapes(n: number): { rows: number; cols: number }[] {
  const out = new Map<string, { rows: number; cols: number }>();
  for (let rows = 1; rows <= Math.min(n, 4); rows++) {
    for (const cols of [Math.ceil(n / rows), Math.ceil(n / rows) + 1]) {
      if ((rows - 1) * cols >= n) continue;
      out.set(`${rows}x${cols}`, { rows, cols });
    }
  }
  return [...out.values()];
}

/**
 * 並べ方の候補を格子の形ごとに 1 つずつ作る。線でよくつながる枠から順に、いちばん見積もり（estimate）が小さくなる
 * 升目へ置き、そのあと升目の入れ替え・枠の中の並びの入れ替え・枠の中の列の数の変更を、見積もりが下がらなくなるまで
 * 繰り返す（局所探索）。部品の ID・名前・役割は見ない（電源の枠を左に置く、といった決まりも置かない）。
 */
export function arrangements(
  model: DiagramModel,
  tree: Tree,
  sizeOf: (id: string) => Size,
  unlinked: readonly Size[],
): (Arrangement & { estimate: number })[] {
  const ends = wireEnds(model);
  const cards = model.cards
    .filter((c) => tree.parent.has(c.id) && !tree.assemblies.has(c.id))
    .map((c) => c.id);
  const rootOf = new Map<string, string>();
  const mark = (id: string, root: string) => {
    rootOf.set(id, root);
    for (const k of tree.children.get(id) ?? []) mark(k, root);
  };
  for (const r of tree.roots) mark(r, r);
  // 枠どうしの線の本数（置く順を決める）
  const weight = new Map<string, number>();
  const pair = (a: string, b: string) => (a < b ? `${a}|${b}` : `${b}|${a}`);
  for (const { a, b } of ends) {
    const ra = rootOf.get(a);
    const rb = rootOf.get(b);
    if (ra && rb && ra !== rb) weight.set(pair(ra, rb), (weight.get(pair(ra, rb)) ?? 0) + 1);
  }
  const w = (a: string, b: string) => weight.get(pair(a, b)) ?? 0;
  return shapes(tree.roots.length).map((shape) => {
    const arr: Arrangement = {
      ...shape,
      slot: new Map(),
      packing: { order: new Map(tree.children), columns: new Map() },
    };
    const frames = [...tree.children.keys()].filter(
      (id) => (tree.children.get(id)?.length ?? 0) > 1,
    );
    const cost = () => estimate(place(tree, arr, sizeOf), ends, unlinked, frames, cards);
    seed(arr, tree.roots, w, cost);
    return { ...arr, estimate: improve(arr, tree, cost) };
  });
}

/** 最初の置き方: 線のいちばん多い枠を中ほどの升目に置き、残りは置いた枠との線の多い順に、見積もりの小さい升目へ */
function seed(
  arr: Arrangement,
  roots: string[],
  w: (a: string, b: string) => number,
  cost: () => number,
) {
  const total = (id: string) => roots.reduce((s, r) => s + w(id, r), 0);
  const left = [...roots];
  const first = left.reduce((best, r) => (total(r) > total(best) ? r : best), left[0] ?? "");
  arr.slot.set(first, Math.floor((arr.rows - 1) / 2) * arr.cols + Math.floor((arr.cols - 1) / 2));
  left.splice(left.indexOf(first), 1);
  while (left.length > 0) {
    const placed = [...arr.slot.keys()];
    const next = left.reduce((best, r) => {
      const score = (id: string) => placed.reduce((s, p) => s + w(id, p), 0);
      return score(r) > score(best) ? r : best;
    }, left[0] ?? "");
    left.splice(left.indexOf(next), 1);
    const used = new Set(arr.slot.values());
    let bestSlot = -1;
    let bestCost = Number.POSITIVE_INFINITY;
    for (let s = 0; s < arr.rows * arr.cols; s++) {
      if (used.has(s)) continue;
      arr.slot.set(next, s);
      const c = cost();
      if (c < bestCost) {
        bestCost = c;
        bestSlot = s;
      }
    }
    arr.slot.set(next, bestSlot);
  }
}

/**
 * 枠の中の格子の形として使ってよいか。カードを縦 1 列に積む形（行が列 + 1 より多い）と、横 1 行に長く並べる形
 * （列が行の 2 倍 + 1 より多い）は使わない。交差を減らそうとすると、2 つの中心（配電とデータの中心）の間に
 * 機器を 1 列に積む形が選ばれやすいが、図全体が列に見えるため。
 */
export function compactShape(n: number, cols: number): boolean {
  const rows = Math.ceil(n / cols);
  return rows <= cols + 1 && cols <= 2 * rows + 1;
}

/** 局所探索。見積もりが下がる変更だけを残し、下がらなくなったら終わる。最後の見積もりを返す */
function improve(arr: Arrangement, tree: Tree, cost: () => number): number {
  let current = cost();
  const attempt = (apply: () => void, undo: () => void) => {
    apply();
    const c = cost();
    if (c < current - 1e-6) {
      current = c;
      return true;
    }
    undo();
    return false;
  };
  for (let round = 0; round < ROUNDS; round++) {
    let changed = false;
    // 升目の入れ替え（空いた升目への移動も含む）
    const n = arr.rows * arr.cols;
    for (let i = 0; i < n; i++)
      for (let j = i + 1; j < n; j++) {
        const at = (s: number) => [...arr.slot].find(([, v]) => v === s)?.[0];
        const a = at(i);
        const b = at(j);
        if (!a && !b) continue;
        const swap = () => {
          if (a) arr.slot.set(a, j);
          if (b) arr.slot.set(b, i);
        };
        const back = () => {
          if (a) arr.slot.set(a, i);
          if (b) arr.slot.set(b, j);
        };
        changed = attempt(swap, back) || changed;
      }
    // 枠の中の並びの入れ替えと、列の数
    for (const [id, kids] of arr.packing.order) {
      for (let i = 0; i < kids.length; i++)
        for (let j = i + 1; j < kids.length; j++) {
          const swap = () => {
            const order = [...(arr.packing.order.get(id) ?? [])];
            [order[i], order[j]] = [order[j] as string, order[i] as string];
            arr.packing.order.set(id, order);
          };
          changed = attempt(swap, swap) || changed;
        }
      const before = arr.packing.columns.get(id) ?? defaultColumns(kids.length);
      for (let c = 1; c <= kids.length; c++) {
        if (c === (arr.packing.columns.get(id) ?? before) || !compactShape(kids.length, c))
          continue;
        const prior = arr.packing.columns.get(id) ?? before;
        changed =
          attempt(
            () => arr.packing.columns.set(id, c),
            () => arr.packing.columns.set(id, prior),
          ) || changed;
      }
    }
    if (!changed || !tree.roots.length) break;
  }
  return current;
}
