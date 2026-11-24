import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";
import type { QuantityView, ThermalCouplingRow, ThermalCouplingsTable } from "@/api/types";
import { CouplingsPanel } from "./CouplingsPanel";

const q = (display: string, dimension: string): QuantityView => ({
  value: 1,
  unit: "W/K",
  si: 1,
  dimension,
  display,
});

const row = (b: string, explicit: boolean): ThermalCouplingRow => ({
  kind: "conductive",
  a: "BAT",
  b,
  value: q("0.8 W/K", "ThermalConductance"),
  source: explicit ? "explicit" : "mount",
  source_label: explicit
    ? { label: "書いた値", label_en: "Written value" }
    : { label: "取付の既定値", label_en: "Mount default" },
  explicit,
  contact: null,
});

const table = (available: boolean): ThermalCouplingsTable => ({
  available,
  kinds: [
    {
      kind: "conductive",
      label: { label: "伝導", label_en: "Conduction" },
      dimension: "ThermalConductance",
    },
  ],
  nodes: ["BAT", "CAM", "PANEL_MZ"],
  rows: available ? [row("PANEL_MZ", true), row("CAM", false)] : [],
});

function html(t: ThermalCouplingsTable) {
  return renderToStaticMarkup(
    <QueryClientProvider client={new QueryClient()}>
      <CouplingsPanel table={t} draft={new Map()} onDraft={() => {}} />
    </QueryClientProvider>,
  );
}

describe("CouplingsPanel", () => {
  it("shows the API's value and source label, and offers withdraw only for written values", () => {
    const out = html(table(true));
    expect(out).toContain("0.8 W/K");
    expect(out.match(/Withdraw: BAT – PANEL_MZ|取り消す: BAT – PANEL_MZ/g)).toHaveLength(1);
    expect(out).not.toMatch(/(Withdraw|取り消す): BAT – CAM/);
    expect(out).toMatch(/Written value|書いた値/);
    expect(out).toMatch(/Mount default|取付の既定値/);
  });

  it("explains when the analysis gives no couplings", () => {
    const out = html(table(false));
    expect(out).not.toContain("<table");
  });
});
