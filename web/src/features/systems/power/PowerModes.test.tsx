import { renderToStaticMarkup } from "react-dom/server";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { PowerBudgetView, PowerModeView, SystemModesView } from "@/api/types";
import { setLang } from "@/i18n";
import { qv } from "@/test/fixture";
import { PowerModesView } from "./PowerModes";
import { powerModesOf } from "./use-power-modes";

// SSR has a fixed Japanese hydration snapshot; emulate the client's selected dictionary here.
vi.mock("@/i18n", async (importOriginal) => {
  const actual = await importOriginal<typeof import("@/i18n")>();
  return { ...actual, useT: actual.getT };
});

const budget: PowerBudgetView = {
  supply: qv(60, "W", "Power"),
  order: 30,
  segments: [
    {
      kind: "load",
      system: "comm",
      label: { label: "通信", label_en: "Communication" },
      value: qv(30, "W", "Power"),
      fraction: 0.5,
    },
    {
      kind: "shunt",
      system: null,
      label: { label: "余り（シャント）", label_en: "Surplus (shunt)" },
      value: qv(30, "W", "Power"),
      fraction: 0.5,
    },
  ],
};
const modes: PowerModeView[] = [
  {
    id: "nominal",
    label: "Nominal",
    verdict: "ok",
    sunlit: budget,
    eclipse: null,
    charts: [],
  },
];

afterEach(() => setLang("ja"));

describe("power modes", () => {
  it("draws one bar per mode with API widths and labels in both languages", () => {
    const render = () => renderToStaticMarkup(<PowerModesView modes={modes} />);
    const ja = render();
    expect(ja).toContain("width:50%");
    expect(ja).toContain("60 W");
    expect(ja).toContain("余り（シャント）");
    expect(ja).toContain("余裕の少ない順");
    setLang("en");
    const en = render();
    expect(en).toContain("Surplus (shunt)");
    expect(en).not.toContain("SAP");
  });
});

describe("power modes from the system modes view", () => {
  it("keeps only the modes that carry a power part, in the API order", () => {
    const empty = {
      sunlit: { parts: {}, total: null, available: null, margin: null, verdict: null },
      aocs: null,
      power: null,
    };
    const view: SystemModesView = {
      system: "power",
      missing: [],
      summary: [],
      modes: [
        { id: "A", ...empty, eclipse: empty.sunlit, power: null },
        { id: "nominal", ...empty, eclipse: empty.sunlit, power: modes[0] ?? null },
      ],
    };
    expect(powerModesOf(view).map((m) => m.id)).toEqual(["nominal"]);
  });
});
