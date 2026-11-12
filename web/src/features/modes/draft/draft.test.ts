import { describe, expect, it } from "vitest";
import { fixture } from "@/test/fixture";
import { cellOf, draftCommand, draftCount, type ModeDraft, withCell } from "./draft";

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

  it("保存済みのセルの割合は入力欄の文字列で見せ、同じ文字列に戻せば下書きから消える", () => {
    const saved = table.rows.find((r) => r.part === "TRX")?.cells.NOMINAL;
    expect(saved?.duty).toBeTruthy();
    const cell = cellOf(table, empty, "NOMINAL", "TRX");
    const [state, fraction] = Object.entries(cell?.duty ?? {})[0] ?? [];
    expect(typeof fraction).toBe("string");
    const d = withCell(table, empty, "NOMINAL", "TRX", {
      state: cell?.state ?? "",
      duty: { [state ?? ""]: fraction ?? "" },
    });
    expect(draftCount(d)).toBe(0);
  });

  it("保存は変えたセルだけを `fune mode set` の引数（打った割合のまま）にする", () => {
    const d = withCell(table, empty, "NOMINAL", "TRX", { state: "rx", duty: { tx: "0.25" } });
    expect(draftCommand(table, d)).toEqual({
      command: "mode_set",
      cells: [
        {
          mode: "NOMINAL",
          part: "TRX",
          state: "rx",
          duty: { tx: "0.25" },
          active: null,
          spare_state: null,
          eclipse: false,
        },
      ],
    });
    expect(draftCommand(table, empty)).toBeNull();
  });

  it("未指定にしたセルは state: null で送る", () => {
    const d = withCell(table, empty, "NOMINAL", "RW_X", null);
    expect(draftCommand(table, d)).toEqual({
      command: "mode_set",
      cells: [
        {
          mode: "NOMINAL",
          part: "RW_X",
          state: null,
          duty: {},
          active: null,
          spare_state: null,
          eclipse: false,
        },
      ],
    });
  });
});

it("稼働台数と予備状態を保存用の文字列として保つ", () => {
  const d = withCell(table, empty, "NOMINAL", "RW_X", {
    state: "on",
    duty: {},
    active: "3",
    spare_state: "idle",
  });
  expect(draftCommand(table, d)).toMatchObject({
    cells: [{ active: "3", spare_state: "idle", eclipse: false }],
  });
});
