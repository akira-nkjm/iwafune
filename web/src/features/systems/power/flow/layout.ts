import type {
  FlowEdge,
  FlowEdgeKind,
  FlowNode,
  FlowNodeKind,
  FlowRow,
  FlowView,
} from "@/api/types";

/**
 * フロー図の置き方（表示だけ。カード・行・つながりは API の `FlowView`）。列は 部品（左）・条件と設定（中）・解析（右）で、
 * 列の中は API の並びのまま上から積む。
 * - 畳めるまとまり（`FlowView.folds`）は先頭の `FOLD_SHOWN` 枚だけを出し、残りは「ほか N 件」のカードにまとめる。
 * - カードには書かれた値の行だけを `MAX_ROWS` 行まで出し、残りは「ほか N 件」の行、書かれていない値（既定値を使う）は
 *   「未設定 N 件」の行にまとめる（どちらも全部は詳細の欄に出す）。
 * - 隠したカード・行の線は、まとめたカード・行から引き、同じ始点と終点の線は 1 本にする。
 */

/** まとまりの中で畳まずに出す枚数 */
export const FOLD_SHOWN = 4;

/** カードに出す値の行の上限（超えたら `MAX_ROWS - 1` 行と「ほか N 件」） */
export const MAX_ROWS = 6;

/** カードの寸法 [px]（見出し・行の高さ・幅） */
export const SIZE = {
  header: 40,
  /** 解析のカードの見出し（題名・ID と定常か時系列かの印・説明 1 行） */
  analysisHeader: 56,
  row: 19,
  pad: 6,
  fold: 30,
  width: { part: 236, middle: 244, analysis: 264 },
  gap: 10,
} as const;

/** 列の左端 [px] */
const COLUMN_X = [0, 290, 590] as const;
type Column = 0 | 1 | 2;

const COLUMN_OF: Record<FlowNodeKind, Column> = {
  part: 0,
  setting: 1,
  modes: 1,
  case: 1,
  analysis: 2,
};

/** カードに出す行（値の行・「ほか N 件」・「未設定 N 件」） */
export type ShownRow =
  | { kind: "row"; key: string; row: FlowRow }
  | { kind: "more"; key: string; count: number }
  | { kind: "unset"; key: string; count: number };

/** まとめた行の鍵（行の ID と重ならない） */
export const MORE_ROW = "~more";
export const UNSET_ROW = "~unset";

/** 置いたカード（`fold` は「ほか N 件」のカード） */
export type LaidCard =
  | {
      kind: "card";
      id: string;
      node: FlowNode;
      rows: ShownRow[];
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

/** 引く線（`sourceRow` は出した行の鍵。無ければカードの出口から） */
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

/** カードに出す行。書かれた値（求めた値を含む）を上限まで、残りと書かれていない値はまとめた行に */
export function shownRows(node: FlowNode): ShownRow[] {
  const set = node.rows.filter((r) => r.set);
  const unset = node.rows.length - set.length;
  const fits = set.length <= MAX_ROWS;
  const head = fits ? set : set.slice(0, MAX_ROWS - 1);
  const out: ShownRow[] = head.map((row) => ({ kind: "row", key: row.id, row }));
  if (!fits) out.push({ kind: "more", key: MORE_ROW, count: set.length - head.length });
  if (unset > 0) out.push({ kind: "unset", key: UNSET_ROW, count: unset });
  return out;
}

/** 行の ID → それを出した行の鍵（隠した行はまとめた行） */
function rowKeys(node: FlowNode, shown: ShownRow[]): Map<string, string> {
  const visible = new Set(shown.flatMap((r) => (r.kind === "row" ? [r.key] : [])));
  return new Map(
    node.rows.map((r) => [r.id, visible.has(r.id) ? r.id : r.set ? MORE_ROW : UNSET_ROW]),
  );
}

/** カードの見出しの高さ（解析のカードは説明の 1 行の分だけ高い） */
export function headerHeight(node: FlowNode): number {
  return node.kind === "analysis" ? SIZE.analysisHeader : SIZE.header;
}

function cardHeight(node: FlowNode, rows: number): number {
  return headerHeight(node) + Math.max(rows, 1) * SIZE.row + SIZE.pad;
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

/** まとまりの最後に出すカード → そのまとまり（「ほか N 件」をその下に置く） */
function foldsAfter(view: FlowView, expanded: ReadonlySet<string>) {
  const after = new Map<string, { id: string; hidden: number }>();
  for (const f of view.folds) {
    if (f.nodes.length <= FOLD_SHOWN) continue;
    const last = expanded.has(f.id) ? f.nodes[f.nodes.length - 1] : f.nodes[FOLD_SHOWN - 1];
    if (last) after.set(last, { id: f.id, hidden: f.nodes.length - FOLD_SHOWN });
  }
  return after;
}

/** API の図から置き方を作る。`expanded` は開いたまとまりの ID */
export function layoutFlow(view: FlowView, expanded: ReadonlySet<string>): FlowLayout {
  const hidden = hiddenCards(view, expanded);
  const after = foldsAfter(view, expanded);
  const keys = new Map<string, Map<string, string>>();
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
    const rows = shownRows(node);
    keys.set(node.id, rowKeys(node, rows));
    push(column, {
      kind: "card",
      id: node.id,
      node,
      rows,
      x: 0,
      y: 0,
      width,
      height: cardHeight(node, rows.length),
    });
    const fold = after.get(node.id);
    if (fold) {
      const open = expanded.has(fold.id);
      push(column, {
        kind: "fold",
        id: fold.id,
        fold: fold.id,
        hidden: fold.hidden,
        expanded: open,
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
    edges: foldEdges(view.edges, hidden, keys),
    columns: COLUMN_X.map((x) => ({ x })),
  };
}

/**
 * 隠したカードの線を「ほか N 件」のカードから、隠した行の線をまとめた行から引き直し、同じ始点・行・終点の線を 1 本にする。
 * `rows` はカード → 行の ID → 出した行の鍵（無ければ行の ID のまま）
 */
export function foldEdges(
  edges: FlowEdge[],
  hidden: ReadonlyMap<string, string>,
  rows: ReadonlyMap<string, ReadonlyMap<string, string>> = new Map(),
): LaidEdge[] {
  const out = new Map<string, LaidEdge>();
  for (const e of edges) {
    const fold = hidden.get(e.source);
    const source = fold ?? e.source;
    const row =
      fold || !e.source_row ? null : (rows.get(e.source)?.get(e.source_row) ?? e.source_row);
    const id = row === e.source_row && !fold ? e.id : `${source}#${row ?? ""}>${e.target}`;
    if (!out.has(id)) out.set(id, { id, kind: e.kind, source, sourceRow: row, target: e.target });
  }
  return [...out.values()];
}
