import type { FlowEdge, FlowEdgeKind, FlowNode, FlowNodeKind, FlowView } from "@/api/types";

/**
 * フロー図の置き方（表示だけ。カード・行・つながりは API の `FlowView`）。列は 部品（左）・条件と設定（中）・解析（右）で、
 * 列の中は API の並びのまま上から積む。畳めるまとまり（`FlowView.folds`）は先頭の `FOLD_SHOWN` 枚だけを出し、残りは
 * 「ほか N 件」のカードにまとめる（隠したカードの線はそのカードから引き、同じ終点の線は 1 本にする）。
 */

/** まとまりの中で畳まずに出す枚数 */
export const FOLD_SHOWN = 4;

/** カードの寸法 [px]（見出し・行の高さ・幅） */
export const SIZE = {
  header: 44,
  row: 20,
  pad: 8,
  fold: 34,
  width: { part: 232, middle: 248, analysis: 256 },
  gap: 12,
} as const;

/** 列の左端 [px] */
const COLUMN_X = [0, 320, 668] as const;
type Column = 0 | 1 | 2;

const COLUMN_OF: Record<FlowNodeKind, Column> = {
  part: 0,
  setting: 1,
  modes: 1,
  case: 1,
  analysis: 2,
};

/** 置いたカード（`fold` は「ほか N 件」のカード） */
export type LaidCard =
  | {
      kind: "card";
      id: string;
      node: FlowNode;
      x: number;
      y: number;
      width: number;
      height: number;
    }
  | {
      kind: "fold";
      id: string;
      fold: string;
      hidden: number;
      expanded: boolean;
      x: number;
      y: number;
      width: number;
      height: number;
    };

/** 引く線（`sourceRow` が無ければカードの出口から） */
export type LaidEdge = {
  id: string;
  kind: FlowEdgeKind;
  source: string;
  sourceRow: string | null;
  target: string;
};

export type FlowLayout = { cards: LaidCard[]; edges: LaidEdge[]; columns: { x: number }[] };

/** 行の出口の端子の ID（行の右端） */
export const rowHandle = (row: string) => `row:${row}`;

/** カードの中の行の縦の位置（中心）[px] */
export function rowCenter(index: number): number {
  return SIZE.header + index * SIZE.row + SIZE.row / 2;
}

function cardHeight(node: FlowNode): number {
  return SIZE.header + Math.max(node.rows.length, 1) * SIZE.row + SIZE.pad;
}

function widthOf(column: Column): number {
  return column === 0 ? SIZE.width.part : column === 1 ? SIZE.width.middle : SIZE.width.analysis;
}

/** 隠すカード → それをまとめた「ほか N 件」のカードの ID */
function hiddenCards(view: FlowView, expanded: ReadonlySet<string>): Map<string, string> {
  const hidden = new Map<string, string>();
  for (const f of view.folds) {
    if (expanded.has(f.id)) continue;
    for (const id of f.nodes.slice(FOLD_SHOWN)) hidden.set(id, f.id);
  }
  return hidden;
}

/** API の図から置き方を作る。`expanded` は開いたまとまりの ID */
export function layoutFlow(view: FlowView, expanded: ReadonlySet<string>): FlowLayout {
  const hidden = hiddenCards(view, expanded);
  // まとまりの最後に出すカード → そのまとまり（「ほか N 件」をその下に置く）
  const foldAfter = new Map<string, { id: string; hidden: number }>();
  for (const f of view.folds) {
    if (f.nodes.length <= FOLD_SHOWN) continue;
    const open = expanded.has(f.id);
    const last = open ? f.nodes[f.nodes.length - 1] : f.nodes[FOLD_SHOWN - 1];
    if (last) foldAfter.set(last, { id: f.id, hidden: f.nodes.length - FOLD_SHOWN });
  }
  const stacks: LaidCard[][] = [[], [], []];
  const heights = [0, 0, 0];
  const push = (column: Column, card: LaidCard) => {
    stacks[column]?.push({ ...card, x: COLUMN_X[column], y: heights[column] ?? 0 });
    heights[column] = (heights[column] ?? 0) + card.height + SIZE.gap;
  };
  for (const node of view.nodes) {
    if (hidden.has(node.id)) continue;
    const column = COLUMN_OF[node.kind];
    const width = widthOf(column);
    push(column, { kind: "card", id: node.id, node, x: 0, y: 0, width, height: cardHeight(node) });
    const fold = foldAfter.get(node.id);
    if (fold) {
      push(column, {
        kind: "fold",
        id: fold.id,
        fold: fold.id,
        hidden: fold.hidden,
        expanded: expanded.has(fold.id),
        x: 0,
        y: 0,
        width,
        height: SIZE.fold,
      });
    }
  }
  // 列は縦の中央でそろえる（いちばん高い列に合わせる）
  const tallest = Math.max(...heights);
  const cards = stacks.flatMap((stack, i) => {
    const shift = (tallest - (heights[i] ?? 0)) / 2;
    return stack.map((c) => ({ ...c, y: c.y + shift }));
  });
  return {
    cards,
    edges: foldEdges(view.edges, hidden),
    columns: COLUMN_X.map((x) => ({ x })),
  };
}

/** 隠したカードの線を「ほか N 件」のカードから引き直し、同じ始点と終点の線を 1 本にする */
export function foldEdges(edges: FlowEdge[], hidden: ReadonlyMap<string, string>): LaidEdge[] {
  const out = new Map<string, LaidEdge>();
  for (const e of edges) {
    const fold = hidden.get(e.source);
    if (fold) {
      const id = `${fold}>${e.target}`;
      if (!out.has(id))
        out.set(id, { id, kind: e.kind, source: fold, sourceRow: null, target: e.target });
      continue;
    }
    out.set(e.id, {
      id: e.id,
      kind: e.kind,
      source: e.source,
      sourceRow: e.source_row,
      target: e.target,
    });
  }
  return [...out.values()];
}
