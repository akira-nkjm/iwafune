import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { renderToStaticMarkup } from "react-dom/server";
import { MemoryRouter } from "react-router";
import { afterEach, describe, expect, it, vi } from "vitest";
import { keys } from "@/api/keys";
import type { AocsAnalysisState, AocsModeView } from "@/api/types";
import { getT, setLang } from "@/i18n";
import { fixture, qv } from "@/test/fixture";
import { AocsModeCard, AocsModes } from "./AocsModes";

vi.mock("@/i18n", async (importOriginal) => {
  const actual = await importOriginal<typeof import("@/i18n")>();
  return { ...actual, useT: () => actual.getT() };
});
const header = {
  id: "NOMINAL",
  label: "NOMINAL",
  file: "modes/NOMINAL.toml",
  description: null,
  max_duration: qv(45, "min", "Time", 2700),
};

const ok: AocsAnalysisState = { status: "ok", verdict: "ok", missing: [], error: null };
const item: AocsModeView = {
  attitude: {
    law: "sun_pointing",
    fields: [
      { field: "law", path: "analysis.aocs.law.NOMINAL.law", text: "sun_pointing" },
      { field: "axis", path: "analysis.aocs.law.NOMINAL.axis", text: "0 0 1" },
    ],
    axes: [
      { body: "[0, 0, 1]", target: "太陽", target_en: "Sun" },
      { body: "[1, 0, 0]", target: "軌道面の法線", target_en: "Orbit normal" },
    ],
  },
  devices: [
    {
      role: "reaction_wheel",
      label: "RW",
      label_en: "RW",
      active_count: 2,
      total_count: 3,
      devices: [
        {
          part: "RW_X",
          active: true,
          states: [{ state: "peak", count: 2, fraction: qv(25, "%", "Fraction", 0.25) }],
        },
        {
          part: "RW_OFF",
          active: false,
          states: [{ state: "off", count: 1, fraction: qv(100, "%", "Fraction", 1) }],
        },
        { part: "UNKNOWN", active: null, states: [] },
      ],
    },
  ],
  eclipse_devices: [],
  momentum_per_orbit: qv(7.25, "mN m s", "AngularMomentum"),
  unloads_per_day: qv(3.5, "", "Dimensionless"),
  mtq_unload_time: qv(120, "s", "Time"),
  rw: ok,
  unloading: ok,
  sensor_fov: { ...ok, verdict: "warning" },
};

function render(value = item, lang: "ja" | "en" = "ja", connected = false) {
  setLang(lang);
  const qc = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  qc.setQueryData(keys.defs, fixture.defs);
  qc.setQueryData(keys.systemModes("aocs", "[]"), {
    system: "aocs",
    modes: [
      {
        id: header.id,
        sunlit: {},
        eclipse: {},
        aocs: value,
      },
    ],
  });
  return renderToStaticMarkup(
    <MemoryRouter>
      <QueryClientProvider client={qc}>
        {connected ? (
          <AocsModes design={fixture} />
        ) : (
          <AocsModeCard design={fixture} header={header} item={value} />
        )}
      </QueryClientProvider>
    </MemoryRouter>,
  );
}

afterEach(() => setLang("ja"));

describe("AOCS mode cards", () => {
  it("renders API values, axes, device states, muted inactive devices, duration and editing fields", () => {
    const html = render();
    for (const text of [
      "7.25 mN m s",
      "3.5",
      "120 s",
      "45 min",
      "RW_X",
      "peak",
      "25 %",
      "[0, 0, 1]",
      "太陽",
      "軌道面の法線",
    ]) {
      expect(html).toContain(text);
    }
    expect(html).toContain("opacity-50");
    expect(html).toContain(getT().systems.aocsModes.unknownState);
    expect(html).toContain(getT().verdict.warning);
    expect(html).toContain('aria-label="analysis.aocs.law.NOMINAL.law"');
    expect(html).toContain('aria-label="analysis.aocs.law.NOMINAL.axis"');
    expect(html).not.toContain("<table");
  });
  it("connects the AOCS query to per-mode cards", () => {
    expect(render(item, "ja", true)).toContain("7.25 mN m s");
  });
  it("renders English labels and targets", () => {
    const html = render(item, "en");
    expect(html).toContain("Sun");
    expect(html).toContain("Orbit normal");
    expect(html).toContain("Max duration");
    expect(html).toContain("Sensor FoV");
  });
  it("distinguishes missing and failed analyses from numeric values and successful verdicts", () => {
    const html = render({
      ...item,
      momentum_per_orbit: null,
      unloads_per_day: null,
      rw: {
        status: "missing_inputs",
        verdict: null,
        error: null,
        missing: [{ path: "analysis.aocs.law.NOMINAL", part: null, target: null, reason: "law" }],
      },
      sensor_fov: { status: "failed", verdict: null, missing: [], error: "Invalid attitude" },
    });
    expect(html).toContain(getT().systems.status.missing_inputs);
    expect(html).toContain(getT().systems.status.failed);
    expect(html).toContain("Invalid attitude");
    expect(html).not.toContain("7.25 mN m s");
  });
});
