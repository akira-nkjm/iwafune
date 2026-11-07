import { describe, expect, it } from "vitest";
import { fixture } from "@/test/fixture";
import { cellOf, draftCount, draftOps, type ModeDraft, withCell } from "./draft";

const table = fixture.modes;
const empty: ModeDraft = new Map();

describe("モード表の下書き", () => {
  it("セルを変えると下書きに入り、元に戻すと消える", () => {
    const d1 = withCell(table, empty, "SAFE", "RW_X", { state: "off", duty: {} });
    expect(draftCount(d1)).toBe(1);
    expect(cellOf(table, d1, "SAFE", "RW_X")?.state).toBe("off");
    const d2 = withCell(table, d1, "SAFE", "RW_X", null);
    expect(draftCount(d2)).toBe(0);
  });

  it("保存は変えたセルだけを 1 つの SetModeStates にする", () => {
    const d = withCell(table, empty, "NOMINAL", "TRX", { state: "rx", duty: { tx: 0.2 } });
    expect(draftOps(table, d)).toEqual([
      {
        op: "set_mode_states",
        cells: [{ mode: "NOMINAL", part: "TRX", state: "rx", duty: { tx: 0.2 } }],
      },
    ]);
  });

  it("未指定にしたセルは state: null で送る", () => {
    const d = withCell(table, empty, "NOMINAL", "RW_X", null);
    expect(draftOps(table, d)).toEqual([
      { op: "set_mode_states", cells: [{ mode: "NOMINAL", part: "RW_X", state: null, duty: {} }] },
    ]);
  });
});
