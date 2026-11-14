import { ReactFlowProvider } from "@xyflow/react";
import type { ReactNode } from "react";
import { renderToStaticMarkup } from "react-dom/server";
import { MemoryRouter } from "react-router";
import { describe, expect, it, vi } from "vitest";
import type { AnalysisRunResult, CheckView } from "@/api/types";
import { flowNode, flowRow, smallFlow } from "@/test/flow";
import { flowEdges, flowNodes } from "./elements";
import { FlowCard, FlowFoldCard } from "./FlowCard";
import { FlowDetail } from "./FlowDetail";
import { focusOf } from "./focus";
import { layoutFlow } from "./layout";

vi.mock("@/i18n", async (importOriginal) => {
  const actual = await importOriginal<typeof import("@/i18n")>();
  return { ...actual, useT: actual.getT };
});

const html = (node: ReactNode) =>
  renderToStaticMarkup(
    <MemoryRouter>
      <ReactFlowProvider>{node}</ReactFlowProvider>
    </MemoryRouter>,
  );

/** React Flow がノードに渡す props のうち、カードが使うもの */
function nodeProps<D, T extends string = "flowCard">(id: string, data: D, type = "flowCard" as T) {
  return {
    id,
    data,
    width: 240,
    height: 120,
    type,
    dragging: false,
    zIndex: 0,
    selectable: false,
    deletable: false,
    selected: false,
    draggable: false,
    isConnectable: false,
    positionAbsoluteX: 0,
    positionAbsoluteY: 0,
  } as const;
}

describe("FlowCard", () => {
  it("名前・ID の補足・値の行を出し、書かれていない値は「未設定」", () => {
    const node = flowNode(
      "part:SP",
      "part",
      [flowRow("area", "800 cm²"), flowRow("eol_factor", null)],
      {
        sub: "solar_panel",
      },
    );
    const out = html(
      <FlowCard
        {...nodeProps("part:SP", {
          node,
          emphasis: "normal",
          rows: new Set<string>(),
          selected: false,
          onRowHover: () => {},
        })}
      />,
    );
    expect(out).toContain("part:SP の名前");
    expect(out).toContain("solar_panel");
    expect(out).toContain("800 cm²");
    expect(out).toContain("未設定");
    expect(out).toContain('data-handleid="row:area"');
  });

  it("解析のカードは判定のバッジと入口・結果の出入口を持ち、薄くするときは透かす", () => {
    const node = flowNode(
      "analysis:balance",
      "analysis",
      [flowRow("max_dod", "15 %", { verdict: "ok" })],
      {
        verdict: "violation",
      },
    );
    const out = html(
      <FlowCard
        {...nodeProps("analysis:balance", {
          node,
          emphasis: "dim",
          rows: new Set(["max_dod"]),
          selected: true,
          onRowHover: () => {},
        })}
      />,
    );
    expect(out).toContain("違反");
    expect(out).toContain('data-handleid="in"');
    expect(out).toContain('data-handleid="res-out"');
    expect(out).toContain("opacity-35");
    expect(out).toContain("bg-accent-tint");
    expect(out).toContain("text-pos-ink");
  });

  it("「ほか N 件」のカードは隠した数、開いていれば「畳む」", () => {
    const closed = html(
      <FlowFoldCard
        {...nodeProps(
          "fold:0",
          { hidden: 3, expanded: false, emphasis: "normal", onToggle: () => {} },
          "flowFold",
        )}
      />,
    );
    expect(closed).toContain("ほか 3 件");
    const open = html(
      <FlowFoldCard
        {...nodeProps(
          "fold:0",
          { hidden: 3, expanded: true, emphasis: "normal", onToggle: () => {} },
          "flowFold",
        )}
      />,
    );
    expect(open).toContain("畳む");
  });
});

describe("flowNodes / flowEdges", () => {
  const laid = layoutFlow(smallFlow(), new Set());
  const on = { onRowHover: () => {}, onToggleFold: () => {} };

  it("列の見出しとカード・「ほか N 件」をノードにし、選んだカードに印を付ける", () => {
    const nodes = flowNodes(laid, null, "part:SP", ["部品", "条件", "解析"], on);
    expect(nodes.filter((n) => n.type === "flowHeading").map((n) => n.data.label)).toEqual([
      "部品",
      "条件",
      "解析",
    ]);
    expect(nodes.find((n) => n.id === "part:SP")?.data).toMatchObject({ selected: true });
    expect(nodes.some((n) => n.type === "flowFold")).toBe(true);
  });

  it("値の線は行の出口から解析の入口へ、結果の線は破線で右の出入口どうし。強調以外は薄く", () => {
    const focus = focusOf(laid.edges, { kind: "row", card: "part:SP", row: "area" });
    const edges = flowEdges(laid.edges, focus);
    const area = edges.find((e) => e.id === "part:SP#area>analysis:balance");
    expect(area).toMatchObject({ sourceHandle: "row:area", targetHandle: "in" });
    expect(area?.style?.strokeWidth).toBe(2.2);
    const result = edges.find((e) => e.id === "analysis:load>analysis:balance");
    expect(result).toMatchObject({ sourceHandle: "res-out", targetHandle: "res-in" });
    expect(result?.style?.strokeDasharray).toBe("5 3");
    expect(result?.style?.opacity).toBe(0.15);
    const folded = edges.find((e) => e.source === "fold:0");
    expect(folded?.sourceHandle).toBe("out");
  });
});

describe("FlowDetail", () => {
  const check: CheckView = {
    key: "dod",
    label: "最大 DoD",
    label_en: null,
    verdict: "violation",
    message: "上限を超えています",
    message_en: null,
    at: "/checks/0",
    scope: null,
  };
  const result = {
    checks: [check],
    missing: [{ part: "BAT", path: "BAT.capacity", target: null, reason: "電池の値" }],
  } as unknown as AnalysisRunResult;

  it("解析は主な出力・判定の文・足りない入力・つながるカード", () => {
    const view = smallFlow();
    const node = view.nodes.find((n) => n.id === "analysis:balance");
    if (!node) throw new Error("no node");
    const out = html(
      <FlowDetail
        node={node}
        result={result}
        from={view.nodes.filter((n) => n.id === "part:SP" || n.id === "analysis:load")}
        to={[]}
        design={undefined}
        onSelect={() => {}}
        onClose={() => {}}
      />,
    );
    expect(out).toContain("主な出力");
    expect(out).toContain("15 %");
    expect(out).toContain("上限を超えています");
    expect(out).toContain("BAT.capacity");
    expect(out).toContain("読む値のカード");
    expect(out).toContain("part:SP の名前");
    expect(out).toContain("結果を使う解析");
  });

  it("値のカードは全部の値と、読む解析", () => {
    const view = smallFlow();
    const node = view.nodes.find((n) => n.id === "part:SP");
    if (!node) throw new Error("no node");
    const out = html(
      <FlowDetail
        node={node}
        result={undefined}
        from={[]}
        to={view.nodes.filter((n) => n.id === "analysis:balance")}
        design={undefined}
        onSelect={() => {}}
        onClose={() => {}}
      />,
    );
    expect(out).toContain("800 cm²");
    expect(out).toContain("未設定");
    expect(out).toContain("この値を読む解析");
    expect(out).not.toContain("判定");
  });
});
