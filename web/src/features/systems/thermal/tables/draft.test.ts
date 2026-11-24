import { describe, expect, it } from "vitest";
import type { ThermalCellText } from "@/api/types";
import {
  cellKey,
  coatingCell,
  draftCommand,
  type ThermalDraft,
  withCell,
  withoutCell,
} from "./draft";

const coupling = (value: string | null): ThermalCellText => ({
  cell: "coupling",
  kind: "conductive",
  a: "BAT",
  b: "CAM",
  value,
});

describe("thermal draft", () => {
  it("replaces an earlier edit of the same target and keeps its position", () => {
    let draft: ThermalDraft = new Map();
    draft = withCell(draft, coupling("0.3 W/K"));
    draft = withCell(draft, coatingCell("grey", "label", "Grey"));
    draft = withCell(draft, coupling(null));
    expect(draft.size).toBe(2);
    expect([...draft.values()][0]).toEqual(coupling(null));
  });

  it("keeps one cell per coating field", () => {
    const a = cellKey(coatingCell("grey", "absorptivity", "0.5"));
    const e = cellKey(coatingCell("grey", "emissivity", "0.7"));
    const l = cellKey(coatingCell("grey", "label", "Grey"));
    expect(new Set([a, e, l]).size).toBe(3);
    expect(cellKey(coatingCell("grey", "absorptivity", "0.9"))).toBe(a);
  });

  it("builds the thermal_set command from the changed cells, none when empty", () => {
    expect(draftCommand(new Map())).toBeNull();
    const draft = withCell(new Map(), coupling("0.3 W/K"));
    expect(draftCommand(draft)).toEqual({ command: "thermal_set", cells: [coupling("0.3 W/K")] });
    const back = withoutCell(draft, cellKey(coupling(null)));
    expect(back.size).toBe(0);
  });
});
