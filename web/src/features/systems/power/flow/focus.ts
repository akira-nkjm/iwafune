import type { LaidEdge } from "./layout";

/** マウスを載せているもの（カード・行・線） */
export type FlowHover =
  | { kind: "card"; id: string }
  | { kind: "row"; card: string; row: string }
  | { kind: "edge"; id: string }
  | null;

/** 強調するもの（カード・線・カードごとの行）。何も載せていなければ null（全部をふつうに出す） */
export type FlowFocus = {
  cards: Set<string>;
  edges: Set<string>;
  rows: Map<string, Set<string>>;
};

/** 載せているものにつながるカード・線・行。カードならその出入りの線、行ならその行の線、線ならその線と両端 */
export function focusOf(edges: readonly LaidEdge[], hover: FlowHover): FlowFocus | null {
  if (!hover) return null;
  const picked = edges.filter((e) => {
    if (hover.kind === "edge") return e.id === hover.id;
    if (hover.kind === "row") return e.source === hover.card && e.sourceRow === hover.row;
    return e.source === hover.id || e.target === hover.id;
  });
  const focus: FlowFocus = { cards: new Set(), edges: new Set(), rows: new Map() };
  if (hover.kind === "card") focus.cards.add(hover.id);
  if (hover.kind === "row") {
    focus.cards.add(hover.card);
    focus.rows.set(hover.card, new Set([hover.row]));
  }
  for (const e of picked) {
    focus.edges.add(e.id);
    focus.cards.add(e.source);
    focus.cards.add(e.target);
    if (e.sourceRow) {
      const rows = focus.rows.get(e.source) ?? new Set<string>();
      rows.add(e.sourceRow);
      focus.rows.set(e.source, rows);
    }
  }
  return focus;
}

/** カードにつながるカード（読む側・読まれる側）の ID。詳細の欄の「読む値」「使う解析」に使う */
export function neighbours(
  edges: readonly LaidEdge[],
  id: string,
): { from: string[]; to: string[] } {
  const from = new Set<string>();
  const to = new Set<string>();
  for (const e of edges) {
    if (e.target === id) from.add(e.source);
    if (e.source === id) to.add(e.target);
  }
  return { from: [...from], to: [...to] };
}
