import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";
import type { System, SystemModesView } from "@/api/types";
import { getT } from "@/i18n";
import { fixture, qv } from "@/test/fixture";
import { LightingSwitch } from "./ModesMatrix";
import { ModesTable } from "./ModesTable";
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
  return {
    system,
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
      },
    ],
  };
}

function render(eclipse: boolean, system?: System) {
  const qc = new QueryClient();
  qc.setQueryData(["design", "defs"], fixture.defs);
  const view = system ? modeView(system) : null;
  return renderToStaticMarkup(
    <QueryClientProvider client={qc}>
      <ModesTable
        modes={fixture.modes.modes}
        groups={matrixGroups(fixture, new Map(), view, getT().modes, eclipse, system)}
        view={{ data: view, pending: false, updating: false }}
        system={system}
        eclipse={eclipse}
        updating={false}
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
