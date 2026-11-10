import { describe, expect, it } from "vitest";
import { modesEn } from "@/i18n/modes.en";
import { modesJa } from "@/i18n/modes.ja";
import { fixture } from "@/test/fixture";
import { cellSymbol, mainState, savedCellText, toggleCell } from "./matrix";

describe("保存済みのセルの文言", () => {
  const row = (part: string) => {
    const r = fixture.modes.rows.find((x) => x.part === part);
    if (!r) throw new Error(part);
    return r;
  };
  it("状態と、API が整形した割合を言語ごとに並べる（文言を API から受け取らない）", () => {
    expect(savedCellText(row("TRX"), "NOMINAL", modesJa.modes)).toBe("rx（tx 10 %）");
    expect(savedCellText(row("TRX"), "NOMINAL", modesEn.modes)).toBe("rx (tx 10 %)");
  });
  it("割合の無いセルは状態だけ、書かれていないモードは undefined", () => {
    expect(savedCellText(row("PCDU"), "SAFE", modesJa.modes)).toBe("on");
    expect(savedCellText(row("TRX"), "SAFE", modesJa.modes)).toBeUndefined();
  });
});

describe("セルを押したときの切り替え", () => {
  const wheel = ["off", "idle", "on", "peak"];
  it("オフなら on へ（語彙の並びで idle が先でも）、オンなら off へ", () => {
    expect(toggleCell(wheel, { state: "off", duty: {} })).toEqual({ state: "on", duty: {} });
    expect(toggleCell(wheel, null)).toEqual({ state: "on", duty: {} });
    expect(toggleCell(wheel, { state: "on", duty: {} })).toEqual({ state: "off", duty: {} });
    expect(toggleCell(wheel, { state: "peak", duty: {} })).toEqual({ state: "off", duty: {} });
  });
  it("時間割合は持ち越さない（間欠のオフを押すとオン）", () => {
    expect(toggleCell(wheel, { state: "off", duty: { on: "0.2" } })).toEqual({
      state: "on",
      duty: {},
    });
    expect(toggleCell(wheel, { state: "on", duty: { peak: "0.1" } })).toEqual({
      state: "off",
      duty: {},
    });
  });
  it("on が無い語彙は off 以外の最初の状態。off が無ければオンのまま。語彙が空なら変えない", () => {
    expect(mainState(["off", "rx", "tx"])).toBe("rx");
    expect(toggleCell(["off", "rx", "tx"], null)?.state).toBe("rx");
    expect(toggleCell(["active"], { state: "active", duty: {} })?.state).toBe("active");
    expect(toggleCell([], null)).toBeNull();
  });
});

describe("セルの記号", () => {
  it("時間割合があれば間欠、off と未指定は横線", () => {
    expect(cellSymbol({ state: "on", duty: {} })).toBe("●");
    expect(cellSymbol({ state: "off", duty: { on: "0.2" } })).toBe("◐");
    expect(cellSymbol({ state: "off", duty: {} })).toBe("—");
    expect(cellSymbol(null)).toBe("—");
  });
});

describe("セルの読み上げ", () => {
  it("部品・モード・状態と、サーバーが整形した電力を含む（日英）", () => {
    expect(modesEn.modes.cellLabel("RW_X", "MISSION", "on", "600 mW")).toBe(
      "RW_X in MISSION: on, 600 mW",
    );
    expect(modesJa.modes.cellLabel("RW_X", "MISSION", "on", "600 mW")).toBe(
      "RW_X、MISSION: on、600 mW",
    );
  });
});
