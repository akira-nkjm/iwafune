import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { renderToStaticMarkup } from "react-dom/server";
import { MemoryRouter } from "react-router";
import { afterEach, describe, expect, it, vi } from "vitest";
import { keys } from "@/api/keys";
import type { AocsAnalysisState, AocsModeView } from "@/api/types";
import { getT, setLang } from "@/i18n";
import { fixture, qv } from "@/test/fixture";
import { AocsModeCard, AocsModeDetails, AocsModes } from "./AocsModes";
import { Attitude } from "./Attitude";

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
  unload_times: [{ means: "mtq", label: "MTQ", label_en: "MTQ", time: qv(120, "s", "Time") }],
  rw: ok,
  unloading: ok,
  sensor_fov: { ...ok, verdict: "warning" },
};

type View = "card" | "connected" | "details" | "editor";

function render(value = item, lang: "ja" | "en" = "ja", view: View = "card") {
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
        {view === "connected" && <AocsModes design={fixture} />}
        {view === "card" && <AocsModeCard design={fixture} header={header} item={value} />}
        {view === "details" && <AocsModeDetails design={fixture} item={value} />}
        {view === "editor" && <Attitude item={value.attitude} />}
      </QueryClientProvider>
    </MemoryRouter>,
  );
}

afterEach(() => setLang("ja"));

const idle = {
  role: "magnetorquer" as const,
  label: "MTQ",
  label_en: "MTQ",
  active_count: 0,
  total_count: 3,
  devices: [],
};

describe("AOCS mode cards", () => {
  it("shows only the compared values on the card: duration, law line, role chips and three values", () => {
    const html = render({ ...item, devices: [...item.devices, idle] });
    for (const text of [
      "45 min",
      "[0, 0, 1]",
      "太陽",
      "RW",
      "2/3",
      "MTQ",
      "0/3",
      "7.25 mN m s",
      "3.5",
    ]) {
      expect(html).toContain(text);
    }
    expect(html).toContain(getT().verdict.warning);
    expect(html).toContain(getT().systems.aocsModes.edit);
    expect(html).toContain('aria-expanded="false"');
    expect(html).toContain('data-idle="true"');
    expect(html).toContain("border-dashed");
    // 詳細（部品ごとの状態・MTQ のアンロード時間）と編集欄は閉じている
    expect(html).not.toContain("RW_X");
    expect(html).not.toContain("120 s");
    expect(html).not.toContain('aria-label="analysis.aocs.law.NOMINAL.law"');
    expect(html).not.toContain("<table");
  });
  it("opens per-device states, inactive and unknown devices and analysis states in the details", () => {
    const html = render(item, "ja", "details");
    for (const text of ["RW_X", "peak", "25 %", "120 s", "7.25 mN m s"]) {
      expect(html).toContain(text);
    }
    expect(html).toContain("opacity-50");
    expect(html).toContain(getT().systems.aocsModes.unknownState);
    expect(html).toContain(getT().systems.aocsModes.sensorFov);
  });
  it("lists the unload time of every means the analysis produced, with the means label from the API", () => {
    const rcs = {
      means: "rcs" as const,
      label: "RCS（最悪の向き）",
      label_en: "RCS (worst direction)",
      time: qv(40, "s", "Time"),
    };
    const html = render({ ...item, unload_times: [rcs] }, "ja", "details");
    expect(html).toContain("RCS（最悪の向き）");
    expect(html).toContain("40 s");
    expect(html).not.toContain("120 s");
    const both = render({ ...item, unload_times: [...item.unload_times, rcs] }, "en", "details");
    expect(both).toContain("RCS (worst direction)");
    expect(both).toContain("120 s");
    expect(both).toContain("40 s");
    const none = render({ ...item, unload_times: [] }, "ja", "details");
    expect(none).not.toContain("120 s");
  });
  it("shows the group of parts without a role (role null) as a chip", () => {
    const other = {
      role: null,
      label: "その他",
      label_en: "Other",
      active_count: 1,
      total_count: 1,
      devices: [],
    };
    const html = render({ ...item, devices: [...item.devices, other] });
    expect(html).toContain("その他");
    expect(render({ ...item, devices: [other] }, "en")).toContain("Other");
  });
  it("keeps the axis diagram and SetText editing fields in the law editor", () => {
    const html = render(item, "ja", "editor");
    expect(html).toContain("軌道面の法線");
    expect(html).toContain('aria-label="analysis.aocs.law.NOMINAL.law"');
    expect(html).toContain('aria-label="analysis.aocs.law.NOMINAL.axis"');
  });
  it("puts the basis note once above the cards and lays them out in a row", () => {
    const html = render(item, "ja", "connected");
    expect(html).toContain("7.25 mN m s");
    expect(html.split(getT().systems.aocsModes.basis)).toHaveLength(2);
    expect(html).toContain("xl:grid-cols-4");
  });
  it("renders English labels and targets", () => {
    const html = render(item, "en");
    expect(html).toContain("Sun");
    expect(html).toContain("Max duration");
    expect(html).toContain("FoV verdict");
    expect(html).toContain("Details");
    expect(render(item, "en", "editor")).toContain("Orbit normal");
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
    expect(html).not.toContain("7.25 mN m s");
  });
  it("lists missing inputs and errors in the details", () => {
    const html = render(
      {
        ...item,
        rw: {
          status: "missing_inputs",
          verdict: null,
          error: null,
          missing: [{ path: "analysis.aocs.law.NOMINAL", part: null, target: null, reason: "law" }],
        },
        sensor_fov: { status: "failed", verdict: null, missing: [], error: "Invalid attitude" },
      },
      "ja",
      "details",
    );
    expect(html).toContain("analysis.aocs.law.NOMINAL");
    expect(html).toContain("Invalid attitude");
  });
});
