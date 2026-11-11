import type { DiagramEdge, DiagramModel } from "../model";
import {
  anchorOf,
  type Box,
  type DiagramLayout,
  GRID,
  JUNCTION_ANCHOR,
  type Pt,
  type Side,
  snapDown,
  TITLE_H,
} from "./geometry";
import type { Tree } from "./pack";
import type { Frame, Terminal, Wire } from "./route";

/**
 * 線を引く準備: ジャンクションを置き、線ごとの両端（端子・分かれ口・ジャンクションの点）と通ってよい枠、
 * 避けるカードと枠の見出しを並べる（route.ts に渡す形）。
 */

export const absBox = (layout: DiagramLayout, id: string): Box | null => {
  const p = layout.absolute.get(id);
  const b = layout.boxes.get(id);
  return p && b ? { ...p, width: b.width, height: b.height } : null;
};

/** 見出しの文字の幅の見積もり [px]（13px の太字。全角はおよそ 13px、半角は 8px） */
const textWidth = (s: string) =>
  [...s].reduce((w, ch) => w + ((ch.codePointAt(0) ?? 0) < 0x2000 ? 8 : 13), 0);

/** 線が避ける枠の見出し: 枠は右上の文字、組立品は上の名前の帯（ContainerNode・CardNode の描き方に合わせる） */
function framesOf(model: DiagramModel, tree: Tree, layout: DiagramLayout): Frame[] {
  const depth = (id: string) => {
    let n = 0;
    for (let p = tree.parent.get(id) ?? null; p; p = tree.parent.get(p) ?? null) n += 1;
    return n;
  };
  const out: Frame[] = [];
  for (const c of model.containers) {
    const b = absBox(layout, c.id);
    if (!b) continue;
    const w = textWidth(c.label) + 2 * GRID;
    out.push({
      id: c.id,
      box: b,
      title: { x: b.x + b.width - 12 - w, y: b.y, width: w + 12, height: TITLE_H },
      depth: depth(c.id),
    });
  }
  for (const id of tree.assemblies) {
    const b = absBox(layout, id);
    if (b) out.push({ id, box: b, title: { ...b, height: 36 }, depth: depth(id) });
  }
  return out;
}

/** 線の端の点（端子の点。ポートの無い端はカードかジャンクションの中心） */
function terminalOf(
  layout: DiagramLayout,
  end: DiagramEdge["source"],
  junctions: ReadonlySet<string>,
  tap?: Pt & { side: Side },
): Terminal | null {
  const at = layout.absolute.get(end.node);
  if (!at) return null;
  const port = tap ?? (end.port ? layout.ports.get(end.port) : null);
  if (port)
    return { at: anchorOf({ x: at.x + port.x, y: at.y + port.y }, port.side), side: port.side };
  const b = layout.boxes.get(end.node);
  return {
    at: junctions.has(end.node)
      ? { x: at.x + JUNCTION_ANCHOR.x, y: at.y + JUNCTION_ANCHOR.y }
      : { x: at.x + (b?.width ?? 0) / 2, y: at.y + (b?.height ?? 0) / 2 },
    side: null,
  };
}

/**
 * ジャンクション（中心の無いネットの分岐）を、つながる端子の点の中ほど（縦と横それぞれの中央値）に置く。
 * カードに重なるときは、空いた格子点まで外へずらす。
 */
function placeJunctions(
  model: DiagramModel,
  tree: Tree,
  layout: DiagramLayout,
  junctions: ReadonlySet<string>,
) {
  const cards = model.cards
    .filter((c) => !tree.assemblies.has(c.id))
    .flatMap((c) => absBox(layout, c.id) ?? []);
  const clear = (p: Pt) =>
    cards.every(
      (b) =>
        p.x <= b.x - GRID ||
        p.x >= b.x + b.width + GRID ||
        p.y <= b.y - GRID ||
        p.y >= b.y + b.height + GRID,
    );
  const median = (xs: number[]) => xs.toSorted((a, b) => a - b)[Math.floor(xs.length / 2)] ?? 0;
  for (const j of model.junctions) {
    const pts = model.edges.flatMap((e) => {
      const other = e.source.node === j.id ? e.target : e.target.node === j.id ? e.source : null;
      const t = other ? terminalOf(layout, other, junctions) : null;
      return t ? [t.at] : [];
    });
    if (pts.length === 0) continue;
    const c = {
      x: snapDown(median(pts.map((p) => p.x))),
      y: snapDown(median(pts.map((p) => p.y))),
    };
    let at = c;
    for (let ring = 1; !clear(at) && ring < 40; ring++)
      at =
        [
          { x: c.x + ring * GRID, y: c.y },
          { x: c.x - ring * GRID, y: c.y },
          { x: c.x, y: c.y + ring * GRID },
          { x: c.x, y: c.y - ring * GRID },
        ].find(clear) ?? at;
    const abs = { x: at.x - JUNCTION_ANCHOR.x, y: at.y - JUNCTION_ANCHOR.y };
    const parent = j.parent ? layout.absolute.get(j.parent) : null;
    layout.absolute.set(j.id, abs);
    layout.boxes.set(j.id, {
      x: abs.x - (parent?.x ?? 0),
      y: abs.y - (parent?.y ?? 0),
      width: JUNCTION_ANCHOR.x,
      height: JUNCTION_ANCHOR.x,
    });
  }
}

/** 線の端が入っている枠と組立品（その線が自由に通ってよい枠） */
function ancestors(tree: Tree, model: DiagramModel, id: string): string[] {
  const junction = model.junctions.find((j) => j.id === id);
  const out: string[] = [id];
  for (
    let p = junction ? junction.parent : (tree.parent.get(id) ?? null);
    p;
    p = tree.parent.get(p) ?? null
  )
    out.push(p);
  return out;
}

/** 線を引く準備をする。`layout` にジャンクションの位置を書き足し、経路探索に渡す線・カード・枠を返す */
export function wiresOf(
  model: DiagramModel,
  tree: Tree,
  layout: DiagramLayout,
): { wires: Wire[]; cards: Box[]; frames: Frame[] } {
  const junctions = new Set(model.junctions.map((j) => j.id));
  placeJunctions(model, tree, layout, junctions);
  const wires = model.edges.flatMap((e): Wire[] => {
    const tap = layout.taps.get(e.id);
    const from = terminalOf(layout, e.source, junctions, tap?.source);
    const to = terminalOf(layout, e.target, junctions, tap?.target);
    if (!from || !to) return [];
    return [
      {
        id: e.id,
        net: e.nets.toSorted().join("+") || e.id,
        from,
        to,
        frames: new Set([
          ...ancestors(tree, model, e.source.node),
          ...ancestors(tree, model, e.target.node),
        ]),
      },
    ];
  });
  const cards = model.cards
    .filter((c) => !tree.assemblies.has(c.id))
    .flatMap((c) => absBox(layout, c.id) ?? []);
  return { wires, cards, frames: framesOf(model, tree, layout) };
}
