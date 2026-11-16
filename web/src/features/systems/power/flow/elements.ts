import type { Edge, Node } from "@xyflow/react";
import { uiHex } from "@/lib/tone";
import type { Emphasis, FlowCardType, FlowFoldType, FlowHeadingType } from "./FlowCard";
import type { FlowFocus } from "./focus";
import type { FlowLayout, LaidEdge } from "./layout";
import { rowHandle } from "./layout";

/** 列の見出しの高さ（カードの上に置く）[px] */
const HEADING_ABOVE = 26;

const EMPTY: ReadonlySet<string> = new Set();

export type FlowHandlers = {
  onRowHover: (card: string, row: string | null) => void;
  onToggleFold: (fold: string) => void;
};

function emphasisOf(focus: FlowFocus | null, id: string): Emphasis {
  if (!focus) return "normal";
  return focus.cards.has(id) ? "focus" : "dim";
}

/** 置き方 → React Flow のノード（カード・「ほか N 件」・列の見出し）。強調と選択を載せる */
export function flowNodes(
  laid: FlowLayout,
  focus: FlowFocus | null,
  selected: string | null,
  headings: readonly string[],
  on: FlowHandlers,
): Node[] {
  const top = Math.min(0, ...laid.cards.map((c) => c.y));
  const heads: FlowHeadingType[] = laid.columns.map((c, i) => ({
    id: `heading:${i}`,
    type: "flowHeading",
    position: { x: c.x, y: top - HEADING_ABOVE },
    data: { label: headings[i] ?? "" },
    draggable: false,
    selectable: false,
  }));
  const cards: Node[] = laid.cards.map((c) => {
    const base = {
      id: c.id,
      position: { x: c.x, y: c.y },
      width: c.width,
      height: c.height,
      // 大きさは置き方で決まっている。`measured` を渡さないと、React Flow はノードを作り直すたび（載せる・選ぶたび）に
      // 測った端子の位置を捨て（`parseHandles`）、測り直すまで線を描けない（載せて離すと線が消えたままになる）
      measured: { width: c.width, height: c.height },
      draggable: false,
      connectable: false,
    };
    if (c.kind === "fold") {
      const fold: FlowFoldType = {
        ...base,
        type: "flowFold",
        data: {
          hidden: c.hidden,
          expanded: c.expanded,
          emphasis: emphasisOf(focus, c.id),
          onToggle: () => on.onToggleFold(c.fold),
        },
      };
      return fold;
    }
    const card: FlowCardType = {
      ...base,
      type: "flowCard",
      data: {
        node: c.node,
        shown: c.rows,
        emphasis: emphasisOf(focus, c.id),
        rows: focus?.rows.get(c.id) ?? EMPTY,
        selected: selected === c.id,
        onRowHover: (row) => on.onRowHover(c.id, row),
      },
    };
    return card;
  });
  return [...heads, ...cards];
}

/** 置き方の線 → React Flow の線。値の線は細い実線、解析の結果の線は破線。強調した線はアクセント色で太く */
export function flowEdges(edges: readonly LaidEdge[], focus: FlowFocus | null): Edge[] {
  return edges.map((e) => {
    const lit = focus?.edges.has(e.id) ?? false;
    const dim = focus !== null && !lit;
    const result = e.kind === "result";
    return {
      id: e.id,
      source: e.source,
      target: e.target,
      sourceHandle: e.sourceRow ? rowHandle(e.sourceRow) : result ? "res-out" : "out",
      targetHandle: result ? "res-in" : "in",
      type: "default",
      zIndex: lit ? 1 : 0,
      focusable: false,
      style: {
        stroke: lit ? uiHex.accent : result ? uiHex.ink3 : uiHex.ink4,
        strokeWidth: lit ? 2.2 : result ? 1.6 : 1,
        strokeDasharray: result ? "5 3" : undefined,
        opacity: dim ? 0.15 : result ? 0.9 : 0.55,
      },
    };
  });
}
