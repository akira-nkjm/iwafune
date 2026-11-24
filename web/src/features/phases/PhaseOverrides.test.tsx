import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { renderToStaticMarkup } from "react-dom/server";
import { MemoryRouter } from "react-router";
import { afterEach, describe, expect, it, vi } from "vitest";
import { keys } from "@/api/keys";
import type { PhaseView } from "@/api/types";
import { setLang } from "@/i18n";
import { fixture, qv } from "@/test/fixture";
import { LawOverrides, OrbitOverride } from "./PhaseOverrides";

vi.mock("@/i18n", async (importOriginal) => {
  const actual = await importOriginal<typeof import("@/i18n")>();
  return { ...actual, useT: () => actual.getT() };
});

const altitude = {
  type: "scalar" as const,
  nominal: qv(250, "km", "Length", 250000),
  min: null,
  max: null,
};

const phase: PhaseView = {
  id: "DISPOSAL",
  position: 3,
  order: 3,
  label: "Disposal",
  start: null,
  end: null,
  duration: null,
  modes: ["SAFE"],
  orbit_overridden: true,
  orbit_from: null,
  orbit: [
    {
      path: "orbit.altitude",
      key: "altitude",
      display: "250 km",
      value: altitude,
      text: "250 km",
      choices: [],
    },
  ],
  laws: [
    {
      mode: "SAFE",
      fields: [],
      attitude: {
        law: "sun_pointing",
        fields: [
          { field: "law", path: "law.SAFE.law", text: "sun_pointing" },
          { field: "axis", path: "law.SAFE.axis", text: "[0, 0, 1]" },
        ],
        axes: [{ body: "[0, 0, 1]", target: "太陽", target_en: "Sun" }],
      },
    },
  ],
  cases: [],
  text: { id: "DISPOSAL" },
  field_paths: {},
  file: "phases/DISPOSAL.toml",
};

function render(node: React.ReactNode, lang: "ja" | "en" = "en") {
  setLang(lang);
  const qc = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  qc.setQueryData(keys.defs, fixture.defs);
  return renderToStaticMarkup(
    <MemoryRouter>
      <QueryClientProvider client={qc}>{node}</QueryClientProvider>
    </MemoryRouter>,
  );
}

afterEach(() => setLang("ja"));

describe("軌道の上書きの編集", () => {
  it("上書きがあれば、欄を設計の軌道と同じ設定の表に出し、写し直しと削除を出す", () => {
    const html = render(<OrbitOverride phase={phase} />);
    expect(html).toContain("orbit.altitude");
    expect(html).toContain("250 km");
    expect(html).toContain("Copy the design orbit again");
    expect(html).toContain("Remove override");
  });

  it("API が選択肢を返した欄（出どころ・遷移のマヌーバ）は選択欄にし、いまの値を選んでおく", () => {
    const text = (value: string) => ({ type: "text" as const, value });
    const unset = { type: "unset" as const };
    const html = render(
      <OrbitOverride
        phase={{
          ...phase,
          orbit_from: "MARS_TRANSFER",
          orbit: [
            {
              path: "orbit.source",
              key: "source",
              display: "transfer",
              value: text("transfer"),
              text: "transfer",
              choices: [
                { value: "elements", label: "ケプラー要素", label_en: "Kepler elements" },
                { value: "transfer", label: "日付つきの遷移", label_en: "Dated transfer" },
              ],
            },
            {
              path: "orbit.maneuver",
              key: "maneuver",
              display: "—",
              value: unset,
              text: "",
              choices: [{ value: "MARS_TRANSFER", label: "Earth → Mars", label_en: null }],
            },
          ],
        }}
      />,
    );
    expect(html).toContain('aria-label="orbit.source"');
    expect(html).toContain('<option value="transfer" selected="">');
    expect(html).toContain('<option value="MARS_TRANSFER">');
    // 未入力の欄は「未設定」の選択肢が選ばれている
    expect(html).toContain('aria-label="orbit.maneuver"');
    expect(html).toContain("Not set");
  });

  it("上書きが無ければ、設計の軌道を写して始める操作だけを出す", () => {
    const html = render(
      <OrbitOverride phase={{ ...phase, orbit_overridden: false, orbit: [] }} />,
      "ja",
    );
    expect(html).toContain("設計の軌道を写して上書きを始める");
    expect(html).not.toContain("上書きを削除");
  });
});

describe("姿勢則の上書きの編集", () => {
  it("上書きしたモードの欄（フェーズの欄のパス）と削除を出し、残りのモードを足せる", () => {
    const html = render(<LawOverrides phase={phase} modeIds={["SAFE", "NOMINAL"]} />);
    expect(html).toContain('id="law.SAFE.axis"');
    expect(html).toContain("Remove the attitude law override of mode SAFE");
    // 足せるのは上書きしていないモードだけ
    expect(html).toContain('<option value="NOMINAL">NOMINAL</option>');
    expect(html).not.toContain('<option value="SAFE">SAFE</option>');
    expect(html).toContain("Add override");
  });

  it("上書きが無く、足せるモードも無いときは、その旨を出す", () => {
    const html = render(<LawOverrides phase={{ ...phase, laws: [] }} modeIds={[]} />);
    expect(html).toContain("No overrides");
    expect(html).toContain("No mode is left to override");
  });
});
