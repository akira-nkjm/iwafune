import type { FlowEdge, FlowNode, FlowRow, FlowView } from "@/api/types";

/** テスト用のフロー図の行 */
export function flowRow(id: string, display: string | null, extra: Partial<FlowRow> = {}): FlowRow {
  return {
    id,
    label: id,
    label_en: id,
    display,
    set: display !== null,
    derived: false,
    varies: null,
    verdict: null,
    target: null,
    ...extra,
  };
}

/** テスト用のフロー図のカード */
export function flowNode(
  id: string,
  kind: FlowNode["kind"],
  rows: FlowRow[],
  extra: Partial<FlowNode> = {},
): FlowNode {
  return {
    id,
    kind,
    label: `${id} の名前`,
    label_en: id,
    sub: null,
    system: kind === "analysis" ? "power" : null,
    target: null,
    rows,
    analysis: kind === "analysis" ? id : null,
    verdict: null,
    status: kind === "analysis" ? "ok" : null,
    members: [],
    ...extra,
  };
}

/** 値の線 */
export function inputEdge(source: string, row: string, target: string): FlowEdge {
  return {
    id: `${source}#${row}>${target}`,
    kind: "input",
    source,
    source_row: row,
    target,
  };
}

/**
 * 小さなフロー図: 太陽電池 1 枚・負荷 6 個（畳むまとまり）・電源の設定・解析 2 つ（負荷 → 収支の結果の線）。
 */
export function smallFlow(): FlowView {
  const loads = ["L1", "L2", "L3", "L4", "L5", "L6"].map((id) =>
    flowNode(`part:${id}`, "part", [flowRow("power", "1 W")]),
  );
  const nodes: FlowNode[] = [
    flowNode("part:SP", "part", [flowRow("area", "800 cm²"), flowRow("eol_factor", null)], {
      system: "power",
    }),
    ...loads,
    flowNode("setting:analysis.power", "setting", [flowRow("dod_max", "0.300")]),
    flowNode("analysis:load", "analysis", [flowRow("mode_load", "17 W")]),
    flowNode("analysis:balance", "analysis", [flowRow("max_dod", "15 %", { verdict: "ok" })], {
      verdict: "ok",
    }),
  ];
  const edges: FlowEdge[] = [
    inputEdge("part:SP", "area", "analysis:balance"),
    inputEdge("part:SP", "eol_factor", "analysis:balance"),
    ...loads.flatMap((l) => [
      inputEdge(l.id, "power", "analysis:load"),
      inputEdge(l.id, "power", "analysis:balance"),
    ]),
    inputEdge("setting:analysis.power", "dod_max", "analysis:balance"),
    {
      id: "analysis:load>analysis:balance",
      kind: "result",
      source: "analysis:load",
      source_row: null,
      target: "analysis:balance",
    },
  ];
  return { nodes, edges, folds: [{ id: "fold:0", nodes: loads.map((l) => l.id) }] };
}
