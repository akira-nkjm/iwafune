import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it, vi } from "vitest";
import type { BudgetView, ModeLoadOutput } from "@/api/types";
import { getT } from "@/i18n";
import { formatKey } from "@/quantity/format-batch";
import { fixture, qv } from "@/test/fixture";
import { LightingSwitch } from "./ModesMatrix";
import { ModesTable } from "./ModesTable";
import { matrixGroups } from "./matrix-rows";

const load: ModeLoadOutput = {
  modes: [
    {
      id: "NOMINAL",
      label: "NOMINAL",
      total: 12,
      total_with_growth: 15,
      total_eclipse: 3,
      total_eclipse_with_growth: 4,
      peak_with_growth: 20,
      groups: [],
      parts: [],
      unassigned: [],
      missing_power: [],
      checks: [],
    },
  ],
};
const budget: BudgetView = {
  kind: "power",
  system: "power",
  label: "Power",
  label_en: null,
  case: null,
  limit: null,
  system_margin: qv(0, "%", "Fraction"),
  total: null,
  margin: null,
  verdict: null,
  fill: null,
  rows: [],
  analysis: "power.mode_load",
  mode: null,
  missing: [],
  notes: [],
  modes: [
    {
      node: "NOMINAL",
      label: "NOMINAL",
      label_en: null,
      system: null,
      current: qv(12, "W", "Power"),
      with_growth: qv(15, "W", "Power"),
      allocated: qv(40, "W", "Power"),
      margin: qv(25, "W", "Power"),
      verdict: "ok",
      fill: null,
    },
  ],
};

function render(eclipse: boolean) {
  const qc = new QueryClient();
  qc.setQueryData(formatKey({ si: 4, dimension: "Power", unit: null }), "4 W");
  return renderToStaticMarkup(
    <QueryClientProvider client={qc}>
      <ModesTable
        modes={fixture.modes.modes}
        groups={matrixGroups(fixture, new Map(), load, getT().modes, eclipse)}
        budget={{ data: budget, pending: false, updating: false }}
        load={{ data: load, pending: false, updating: false }}
        eclipse={eclipse}
        updating={false}
        onCell={() => {}}
      />
    </QueryClientProvider>,
  );
}

describe("日照／蝕の行列", () => {
  it("足もとは API の日照の値と蝕の合計を切り替え、蝕で日照の余裕や判定を流用しない", () => {
    const sunlit = render(false);
    expect(sunlit).toContain("15 W");
    expect(sunlit).toContain("40 W");
    expect(sunlit).toContain("25 W");
    const eclipse = render(true);
    expect(eclipse).toContain("4 W");
    expect(eclipse).not.toContain("15 W");
    expect(eclipse).not.toContain("40 W");
    expect(eclipse).not.toContain("25 W");
    expect(eclipse).toContain(getT().modes.eclipsePowerUnavailable);
  });
});

vi.mock("../draft/use-mode-draft-load", () => ({
  useModeDraftLoad: () => ({
    load: { data: load, pending: false, updating: false },
    budget: { data: budget, pending: false, updating: false },
    save: { ops: null, error: null },
  }),
}));

it("切り替えの選択中ボタンを aria-pressed と色で示す", () => {
  const t = getT().modes;
  for (const eclipse of [false, true]) {
    const html = renderToStaticMarkup(<LightingSwitch value={eclipse} onChange={() => {}} />);
    expect(html).toContain(`aria-label="${t.lighting}"`);
    expect(html.match(/aria-pressed="true"/g)).toHaveLength(1);
    expect(html).toContain(`aria-pressed="true"`);
    expect(html).toMatch(
      new RegExp(`aria-pressed="true"[^>]*>${eclipse ? t.eclipseView : t.sunlit}<`),
    );
    expect(html).toContain("aria-pressed:bg-accent/15");
  }
});
