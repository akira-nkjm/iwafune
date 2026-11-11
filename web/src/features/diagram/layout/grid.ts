import type { DiagramModel } from "../model";
import {
  aspectMiss,
  type Box,
  CARD_H,
  CARD_W,
  type DiagramLayout,
  extentOf,
  type Size,
} from "./geometry";

/** 線の無い枠を置く間隔と、枠の中の格子 */
const GAP = 60;
const COLS = 3;
const CELL_GAP = { x: 24, y: 20 };
const PAD = { top: 34, side: 16, bottom: 16 };

/** 線の無い枠とカードの格子: いちばん外のものの大きさと、中のものの位置（親からの相対） */
export interface Unlinked {
  roots: (Size & { id: string })[];
  boxes: Map<string, Box>;
}

/** 線の無い枠（構体など）とカードを格子に並べ、大きさを測る（置く場所はまだ決めない）。枠の中は 3 列の格子（ID 順） */
export function gridUnlinked(model: DiagramModel, inGraph: ReadonlySet<string>): Unlinked {
  const nodes = [...model.containers, ...model.cards];
  const childrenOf = (id: string) =>
    nodes.filter((n) => n.parent === id).sort((a, b) => a.id.localeCompare(b.id));
  const cardIds = new Set(model.cards.map((c) => c.id));
  const boxes = new Map<string, Box>();
  const sum = (xs: number[], n: number) => xs.slice(0, n).reduce((a, b) => a + b, 0);
  const place = (id: string): Size => {
    const kids = childrenOf(id);
    if (kids.length === 0)
      return cardIds.has(id)
        ? { width: CARD_W, height: CARD_H }
        : { width: CARD_W + 2 * PAD.side, height: PAD.top + PAD.bottom };
    const sizes = kids.map((k) => place(k.id));
    const colW = Array.from({ length: Math.min(COLS, kids.length) }, (_, c) =>
      Math.max(...sizes.filter((_, i) => i % COLS === c).map((s) => s.width)),
    );
    const rows = Math.ceil(kids.length / COLS);
    const rowH = Array.from({ length: rows }, (_, r) =>
      Math.max(...sizes.slice(r * COLS, r * COLS + COLS).map((s) => s.height)),
    );
    kids.forEach((k, i) => {
      const c = i % COLS;
      const r = Math.floor(i / COLS);
      boxes.set(k.id, {
        x: PAD.side + sum(colW, c) + c * CELL_GAP.x,
        y: PAD.top + sum(rowH, r) + r * CELL_GAP.y,
        ...(sizes[i] ?? { width: CARD_W, height: CARD_H }),
      });
    });
    return {
      width: 2 * PAD.side + sum(colW, colW.length) + (colW.length - 1) * CELL_GAP.x,
      height: PAD.top + sum(rowH, rows) + (rows - 1) * CELL_GAP.y + PAD.bottom,
    };
  };
  const roots = nodes
    .filter((n) => !n.parent && !inGraph.has(n.id))
    .map((n) => ({ id: n.id, ...place(n.id) }));
  return { roots, boxes };
}

/**
 * 線のある図（\`graph\`）に対して、線の無い枠を下と右のどちらに置くか。全体の縦横比が目標（TARGET_ASPECT）に
 * 近くなる方を選び、そのときの全体の大きさも返す。下なら左から横に、右なら上から縦に並べる。
 */
export function besideOrBelow(graph: Size, unlinked: readonly Size[]): Size & { below: boolean } {
  if (unlinked.length === 0) return { ...graph, below: true };
  const span = (xs: number[]) => xs.reduce((a, b) => a + b, 0) + GAP * (xs.length - 1);
  const row = {
    width: span(unlinked.map((s) => s.width)),
    height: Math.max(...unlinked.map((s) => s.height)),
  };
  const column = {
    width: Math.max(...unlinked.map((s) => s.width)),
    height: span(unlinked.map((s) => s.height)),
  };
  const below = {
    width: Math.max(graph.width, row.width),
    height: graph.height + GAP + row.height,
  };
  const right = {
    width: graph.width + GAP + column.width,
    height: Math.max(graph.height, column.height),
  };
  return aspectMiss(below.width, below.height) <= aspectMiss(right.width, right.height)
    ? { ...below, below: true }
    : { ...right, below: false };
}

/** 線の無い枠を、線のある図の下か右（besideOrBelow）に置く */
export function placeUnlinked(
  model: DiagramModel,
  unlinked: Unlinked,
  layout: DiagramLayout,
): void {
  if (unlinked.roots.length === 0) return;
  for (const [id, box] of unlinked.boxes) layout.boxes.set(id, box);
  const nodes = [...model.containers, ...model.cards];
  const setAbsolute = (id: string, x: number, y: number) => {
    layout.absolute.set(id, { x, y });
    for (const k of nodes.filter((n) => n.parent === id)) {
      const b = layout.boxes.get(k.id);
      if (b) setAbsolute(k.id, x + b.x, y + b.y);
    }
  };
  const graph = extentOf([...layout.absolute.keys()], layout);
  const { below } = besideOrBelow(graph, unlinked.roots);
  let x = below ? graph.x : graph.x + graph.width + GAP;
  let y = below ? graph.y + graph.height + GAP : graph.y;
  for (const root of unlinked.roots) {
    layout.boxes.set(root.id, { x, y, width: root.width, height: root.height });
    setAbsolute(root.id, x, y);
    if (below) x += root.width + GAP;
    else y += root.height + GAP;
  }
}
