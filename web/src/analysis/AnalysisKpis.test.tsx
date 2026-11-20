import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";
import type { AnalysisState } from "@/api/queries";
import type { AnalysisEntry, AnalysisRunResult, KpiView } from "@/api/types";
import { AnalysisKpis } from "./AnalysisKpis";

const entry = (id: string, kpis: AnalysisEntry["kpis"]): AnalysisEntry => ({
  info: {
    id,
    title: id,
    title_en: id,
    area: { kind: "system", value: "thermal" },
    kind: "steady",
    description: "",
    description_en: "",
  },
  input_type: "",
  output_type: "",
  projectable: true,
  kpis,
});

const steady = entry("thermal.steady", [
  {
    key: "max_temperature",
    label: "最高温度",
    label_en: "Max temperature",
    when_measured: false,
    overview: null,
  },
  {
    key: "nodes_in_range",
    label: "温度範囲の判定（節点）",
    label_en: "Range checks",
    when_measured: false,
    overview: null,
  },
]);
const transient = entry("thermal.transient", [
  {
    key: "transient_max_temperature",
    label: "非定常の最高温度",
    label_en: "Transient max",
    when_measured: false,
    overview: null,
  },
]);

const kpis: KpiView[] = [
  {
    key: "max_temperature",
    label: "最高温度",
    label_en: "Max temperature",
    value: { value: 50.5, unit: "°C", si: 323.69, dimension: "Temperature", display: "50.5 °C" },
    verdict: "violation",
    fill: null,
    tally: null,
  },
  {
    key: "nodes_in_range",
    label: "温度範囲の判定（節点）",
    label_en: "Range checks",
    value: null,
    verdict: "violation",
    fill: { ratio: 5 / 9, basis: "ok_share" },
    tally: { checked: 9, flagged: 4 },
  },
];

const result: AnalysisRunResult = {
  id: "thermal.steady",
  case: null,
  case_source: null,
  status: "ok",
  checks: [],
  verdict: "violation",
  kpis,
  input: null,
  output: {},
  dims: {},
  labels: {},
  missing: [],
  error: null,
  overview: { main: [], checks: [], chart: null, hidden_tables: [] },
};

const states: AnalysisState[] = [
  { id: "thermal.steady", result, error: null, loading: false, updating: false },
  { id: "thermal.transient", result: undefined, error: null, loading: true, updating: false },
];

describe("AnalysisKpis", () => {
  it("解析の一覧を待つ間は帯のスケルトン", () => {
    const html = renderToStaticMarkup(<AnalysisKpis entries={[]} states={[]} loading />);
    expect(html).toContain('aria-busy="true"');
    expect(html).not.toContain("<dl");
  });

  it("名前と値は API のものを並べ、数える KPI は数の文、結果を待つ KPI は値のスケルトン", () => {
    const html = renderToStaticMarkup(
      <AnalysisKpis entries={[steady, transient]} states={states} loading={false} />,
    );
    expect(html).toContain("最高温度");
    expect(html).toContain("50.5 °C");
    expect(html).toContain("4 / 9 が要確認");
    expect(html).toContain("非定常の最高温度");
    // 非定常はまだ結果が無いので、その欄だけ読み込み中
    expect(html.match(/aria-busy="true"/g)).toHaveLength(1);
  });
});
