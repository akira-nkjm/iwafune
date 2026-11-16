import { describe, expect, it } from "vitest";
import { flowNode, flowRow, inputEdge, smallFlow } from "@/test/flow";
import {
  FOLD_SHOWN,
  foldEdges,
  layoutFlow,
  MAX_ROWS,
  MORE_ROW,
  rowCenter,
  SIZE,
  shownRows,
  UNSET_ROW,
} from "./layout";

describe("layoutFlow", () => {
  it("部品・条件と設定・解析の 3 列に、API の並びのまま上から積む", () => {
    const laid = layoutFlow(smallFlow(), new Set());
    const x = (id: string) => laid.cards.find((c) => c.id === id)?.x;
    expect(x("part:SP")).toBe(laid.columns[0]?.x);
    expect(x("setting:analysis.power")).toBe(laid.columns[1]?.x);
    expect(x("analysis:load")).toBe(laid.columns[2]?.x);
    const y = (id: string) => laid.cards.find((c) => c.id === id)?.y ?? Number.NaN;
    expect(y("analysis:load")).toBeLessThan(y("analysis:balance"));
    expect(y("part:SP")).toBeLessThan(y("part:L1"));
  });

  it("カードの高さは行の数から決まり、同じ列で重ならない", () => {
    const laid = layoutFlow(smallFlow(), new Set());
    const sp = laid.cards.find((c) => c.id === "part:SP");
    expect(sp?.height).toBe(SIZE.header + 2 * SIZE.row + SIZE.pad);
    const parts = laid.cards.filter((c) => c.x === laid.columns[0]?.x).sort((a, b) => a.y - b.y);
    for (let i = 1; i < parts.length; i++) {
      const [above, below] = [parts[i - 1], parts[i]];
      if (above && below) expect(below.y).toBeGreaterThanOrEqual(above.y + above.height);
    }
  });

  it("まとまりは先頭の FOLD_SHOWN 枚だけを出し、残りを「ほか N 件」にまとめる", () => {
    const laid = layoutFlow(smallFlow(), new Set());
    const loads = laid.cards.filter((c) => c.kind === "card" && c.id.startsWith("part:L"));
    expect(loads.map((c) => c.id)).toEqual(["part:L1", "part:L2", "part:L3", "part:L4"]);
    expect(FOLD_SHOWN).toBe(4);
    const fold = laid.cards.find((c) => c.kind === "fold");
    expect(fold).toMatchObject({ id: "fold:0", hidden: 2, expanded: false });
    // 隠した L5・L6 の線は「ほか N 件」から終点ごとに 1 本
    const fromFold = laid.edges.filter((e) => e.source === "fold:0");
    expect(fromFold.map((e) => e.target).sort()).toEqual(["analysis:balance", "analysis:load"]);
    expect(laid.edges.some((e) => e.source === "part:L5")).toBe(false);
  });

  it("開いたまとまりは全部を出し、最後に「畳む」を置く", () => {
    const laid = layoutFlow(smallFlow(), new Set(["fold:0"]));
    expect(laid.cards.filter((c) => c.id.startsWith("part:L"))).toHaveLength(6);
    const ids = laid.cards.filter((c) => c.x === laid.columns[0]?.x).map((c) => c.id);
    expect(ids.indexOf("fold:0")).toBe(ids.indexOf("part:L6") + 1);
    expect(laid.cards.find((c) => c.kind === "fold")).toMatchObject({ expanded: true });
    expect(laid.edges.some((e) => e.source === "fold:0")).toBe(false);
  });

  it("行の出口は行の中央", () => {
    expect(rowCenter(0)).toBe(SIZE.header + SIZE.row / 2);
    expect(rowCenter(2) - rowCenter(1)).toBe(SIZE.row);
  });
});

describe("foldEdges", () => {
  it("隠さなければ API の線をそのまま（行の出口つき）", () => {
    const view = smallFlow();
    const all = foldEdges(view.edges, new Map());
    expect(all).toHaveLength(view.edges.length);
    expect(all[0]).toMatchObject({ source: "part:SP", sourceRow: "area" });
    expect(all.find((e) => e.kind === "result")?.sourceRow).toBeNull();
  });
});

describe("shownRows", () => {
  const many = flowNode("setting:orbit", "setting", [
    ...["a", "b", "c", "d", "e", "f", "g", "h"].map((id) => flowRow(id, `${id} の値`)),
    flowRow("x", null),
    flowRow("y", null),
  ]);

  it("書かれた値を上限まで出し、残りは「ほか N 件」、書かれていない値は「未設定 N 件」にまとめる", () => {
    const rows = shownRows(many);
    expect(rows).toHaveLength(MAX_ROWS + 1);
    expect(rows.slice(0, MAX_ROWS - 1).map((r) => r.key)).toEqual(["a", "b", "c", "d", "e"]);
    expect(rows[MAX_ROWS - 1]).toMatchObject({ kind: "more", key: MORE_ROW, count: 3 });
    expect(rows[MAX_ROWS]).toMatchObject({ kind: "unset", key: UNSET_ROW, count: 2 });
  });

  it("上限以内なら全部の書かれた値", () => {
    const few = flowNode("p", "part", [flowRow("a", "1"), flowRow("b", "2")]);
    expect(shownRows(few).map((r) => r.kind)).toEqual(["row", "row"]);
  });

  it("隠した行の線はまとめた行から、同じ終点への線は 1 本", () => {
    const view = {
      nodes: [many, flowNode("analysis:a", "analysis", [flowRow("o", "1")])],
      edges: ["a", "g", "h", "x", "y"].map((r) => inputEdge("setting:orbit", r, "analysis:a")),
      folds: [],
    };
    const laid = layoutFlow(view, new Set());
    expect(laid.edges.map((e) => e.sourceRow).sort()).toEqual(["a", MORE_ROW, UNSET_ROW].sort());
  });
});
