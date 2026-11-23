import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";
import type { System, SystemModesView } from "@/api/types";
import { getT } from "@/i18n";
import { fixture, qv } from "@/test/fixture";
import { LightingSwitch } from "./ModesMatrix";
import { ModesTable } from "./ModesTable";
import { modeLevels } from "./matrix";
import { matrixGroups } from "./matrix-rows";

function modeView(system: System = "power"): SystemModesView {
  const unit = system === "cdh" ? "bit/s" : "W";
  const dimension = system === "cdh" ? "DataRate" : "Power";
  const values = {
    parts: { RW_X: qv(12, unit, dimension) },
    total: qv(15, unit, dimension),
    available: qv(40, "W", "Power"),
    margin: qv(25, "W", "Power"),
    verdict: "ok" as const,
  };
  // API の足もとの行（`system_modes::summary_rows`）と同じ
  const summary: SystemModesView["summary"] =
    system === "power"
      ? [
          { label: "power", field: "total", verdict: false },
          { label: "available", field: "available", verdict: false },
          { label: "margin", field: "margin", verdict: true },
        ]
      : [{ label: system === "cdh" ? "data_rate" : "heat", field: "total", verdict: true }];
  return {
    system,
    missing: [],
    summary,
    modes: [
      {
        id: "NOMINAL",
        sunlit: values,
        eclipse: {
          ...values,
          parts: { RW_X: qv(3, unit, dimension) },
          total: qv(4, unit, dimension),
          margin: qv(36, "W", "Power"),
        },
        aocs: null,
        power: null,
      },
    ],
  };
}

/** 設計のモードのページの足もと: API が `summary: "totals"` で返す、合計の行だけの表示 */
function totalsView(): SystemModesView {
  return { ...modeView("power"), summary: [{ label: "power", field: "total", verdict: false }] };
}

function render(eclipse: boolean, system?: System, footer = false) {
  const qc = new QueryClient();
  qc.setQueryData(["design", "defs"], fixture.defs);
  const view = system ? modeView(system) : footer ? totalsView() : null;
  return renderToStaticMarkup(
    <QueryClientProvider client={qc}>
      <ModesTable
        modes={fixture.modes.modes}
        groups={matrixGroups(
          fixture,
          new Map(),
          system ? view : null,
          getT().modes,
          eclipse,
          system,
        )}
        view={{ data: view, pending: false, updating: false }}
        system={system}
        levels={modeLevels(fixture.defs, system)}
        eclipse={eclipse}
        onCell={() => {}}
      />
    </QueryClientProvider>,
  );
}

describe("系別のモードの行列", () => {
  it("電源は API の日照・蝕の部品値、合計、使用可能量、余裕を表示する", () => {
    const sunlit = render(false, "power");
    expect(sunlit).toContain("12 W");
    expect(sunlit).toContain("15 W");
    expect(sunlit).toContain("40 W");
    expect(sunlit).toContain("25 W");
    const eclipse = render(true, "power");
    expect(eclipse).toContain("3 W");
    expect(eclipse).toContain("4 W");
    expect(eclipse).toContain("40 W");
    expect(eclipse).toContain("36 W");
    expect(eclipse).not.toContain("25 W");
    expect(eclipse).not.toContain("15 W");
  });
  it("定義ページには収支とモードの解析値を出さず、状態と単体の電力列を残す", () => {
    const html = render(false);
    expect(html).toContain(getT().modes.powerLevels);
    expect(html).not.toContain(getT().modes.available);
    expect(html).not.toContain(getT().modes.margin);
    expect(html).not.toContain("15 W");
  });
  it("定義ページの足もとは電力の合計の行だけで、日照・蝕の切り替えに従い、判定は付けない", () => {
    const tm = getT().modes;
    const sunlit = render(false, undefined, true);
    expect(sunlit).toContain(tm.powerRow);
    expect(sunlit).toContain("15 W");
    expect(sunlit).not.toContain("4 W");
    expect(sunlit).not.toContain(tm.available);
    expect(sunlit).not.toContain(tm.margin);
    expect(sunlit).not.toContain("40 W");
    // 判定のピルは出さない（`summary` の `verdict` が偽の行）
    expect(sunlit).not.toContain(getT().verdict.ok);
    const eclipse = render(true, undefined, true);
    expect(eclipse).toContain("4 W");
    expect(eclipse).not.toContain("15 W");
    // 本体は今のまま: 単体の電力列があり、系のタブの部品値（12 W）は出ない
    expect(sunlit).toContain(tm.powerLevels);
    expect(sunlit).not.toContain("12 W");
  });
  it("C&DH・熱 はそれぞれの見出しを出し、電力の収支を出さない", () => {
    for (const [system, title] of [
      ["cdh", getT().modes.dataRate],
      ["thermal", getT().modes.heat],
    ] as const) {
      const html = render(false, system);
      expect(html).toContain(title);
      expect(html).not.toContain(getT().modes.available);
      expect(html).not.toContain(getT().modes.powerLevels);
      expect(html).not.toContain(getT().modes.durationAdd);
    }
  });
});

it("切り替えの選択中ボタンを aria-pressed と色で示す", () => {
  const t = getT().modes;
  for (const eclipse of [false, true]) {
    const html = renderToStaticMarkup(<LightingSwitch value={eclipse} onChange={() => {}} />);
    expect(html).toContain(`aria-label="${t.lighting}"`);
    expect(html.match(/aria-pressed="true"/g)).toHaveLength(1);
    expect(html).toMatch(
      new RegExp(`aria-pressed="true"[^>]*>${eclipse ? t.eclipseView : t.sunlit}<`),
    );
  }
});
