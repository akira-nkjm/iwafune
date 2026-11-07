import { type Box, CARD_H, CARD_W, type Size, snapDown, snapUp } from "./layout-geometry";
import type { DiagramModel } from "./model";

/** 枠（サブシステム）の内側の余白（上は見出しの帯の分）[px]。線が枠の縁に沿わずに入れるよう広めにとる */
const FRAME_PAD = { top: 48, side: 24, bottom: 24 };
/** 組立品のカードの内側の余白（上は組立品の名前の分）[px] */
const ASSEMBLY_PAD = { top: 56, side: 16, bottom: 16 };
/** 枠の中のもののすき間 [px]。カードの間を線が 2〜3 本通れる幅 */
const GAP = { x: 56, y: 48 };

/** 自動配置で並べるもの（線のある枠とカード）の親子。順は表示モデルの順 */
export interface Tree {
  /** いちばん外の枠とカード（図の上で縦横に並べる単位） */
  roots: string[];
  children: Map<string, string[]>;
  parent: Map<string, string | null>;
  /** 子を持つカード（組立品）。枠と同じく中に子を並べる */
  assemblies: Set<string>;
  /** 枠（サブシステム） */
  frames: Set<string>;
}

/** 中の並べ方: 枠・組立品ごとの子の順と、格子の列の数 */
export interface Packing {
  order: Map<string, string[]>;
  columns: Map<string, number>;
}

/**
 * 自動配置で並べるのは、線の端にあるカード・ジャンクションと同じいちばん外の枠に入っているもの全部。
 * 線の無い枠（構体など）はここに入らず、図の下か右に格子で並べる（grid.ts）。
 */
export function connectedNodes(model: DiagramModel): Set<string> {
  const parents = new Map<string, string | null>(
    [...model.containers, ...model.cards, ...model.junctions].map((n) => [n.id, n.parent]),
  );
  const rootOf = (id: string) => {
    let r = id;
    for (let p = parents.get(r) ?? null; p; p = parents.get(p) ?? null) r = p;
    return r;
  };
  const roots = new Set(model.edges.flatMap((e) => [rootOf(e.source.node), rootOf(e.target.node)]));
  return new Set([...parents.keys()].filter((id) => roots.has(rootOf(id))));
}

/** 線のある枠とカードの親子（ジャンクションは入れない。置き場所は線が決まってから決める） */
export function treeOf(model: DiagramModel, inGraph: ReadonlySet<string>): Tree {
  const nodes = [...model.containers, ...model.cards].filter((n) => inGraph.has(n.id));
  const children = new Map<string, string[]>();
  const parent = new Map<string, string | null>();
  for (const n of nodes) {
    parent.set(n.id, n.parent);
    if (n.parent) children.set(n.parent, [...(children.get(n.parent) ?? []), n.id]);
  }
  return {
    roots: nodes.filter((n) => !n.parent).map((n) => n.id),
    children,
    parent,
    assemblies: new Set(model.cards.filter((c) => children.has(c.id)).map((c) => c.id)),
    frames: new Set(model.containers.filter((c) => inGraph.has(c.id)).map((c) => c.id)),
  };
}

/** 子が n 個のときの最初の列の数（横長に近くなる数）。探索（layout-place.ts）がここから変える */
export const defaultColumns = (n: number) => Math.max(1, Math.round(Math.sqrt(n * 1.5)));

const offsets = (sizes: number[], start: number, gap: number) => {
  const out: number[] = [];
  let at = start;
  for (const s of sizes) {
    out.push(at);
    at += s + gap;
  }
  return out;
};
const sum = (xs: number[]) => xs.reduce((a, b) => a + b, 0);

/**
 * 枠と組立品の中を格子に並べ、親からの相対の箱を返す（いちばん外のものは位置 0, 0 で大きさだけ）。
 * 列の幅と行の高さは中のものの大きさの最大で、位置はすべて GRID の倍数（端子と線を格子に乗せるため）。
 */
export function pack(tree: Tree, packing: Packing, sizeOf: (id: string) => Size): Map<string, Box> {
  const boxes = new Map<string, Box>();
  const measure = (id: string): Size => {
    const kids = packing.order.get(id) ?? tree.children.get(id) ?? [];
    if (kids.length === 0) {
      return tree.frames.has(id)
        ? { width: CARD_W + 2 * FRAME_PAD.side, height: FRAME_PAD.top + FRAME_PAD.bottom }
        : sizeOf(id);
    }
    const pad = tree.assemblies.has(id) ? ASSEMBLY_PAD : FRAME_PAD;
    const sizes = kids.map(measure);
    const cols = Math.min(
      kids.length,
      Math.max(1, packing.columns.get(id) ?? defaultColumns(kids.length)),
    );
    const rows = Math.ceil(kids.length / cols);
    const colW = Array.from({ length: cols }, () => 0);
    const rowH = Array.from({ length: rows }, () => 0);
    sizes.forEach((s, i) => {
      const c = i % cols;
      const r = Math.floor(i / cols);
      colW[c] = Math.max(colW[c] ?? 0, snapUp(s.width));
      rowH[r] = Math.max(rowH[r] ?? 0, snapUp(s.height));
    });
    const xs = offsets(colW, pad.side, GAP.x);
    const ys = offsets(rowH, pad.top, GAP.y);
    kids.forEach((k, i) => {
      const c = i % cols;
      const r = Math.floor(i / cols);
      const s = sizes[i] ?? { width: CARD_W, height: CARD_H };
      boxes.set(k, {
        x: (xs[c] ?? 0) + snapDown(((colW[c] ?? 0) - s.width) / 2),
        y: (ys[r] ?? 0) + snapDown(((rowH[r] ?? 0) - s.height) / 2),
        width: s.width,
        height: s.height,
      });
    });
    return {
      width: 2 * pad.side + sum(colW) + (cols - 1) * GAP.x,
      height: pad.top + sum(rowH) + (rows - 1) * GAP.y + pad.bottom,
    };
  };
  for (const root of tree.roots) boxes.set(root, { x: 0, y: 0, ...measure(root) });
  return boxes;
}
