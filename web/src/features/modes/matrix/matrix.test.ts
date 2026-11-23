import { describe, expect, it } from "vitest";
import { modesEn } from "@/i18n/modes/en";
import { modesJa } from "@/i18n/modes/ja";
import { fixture } from "@/test/fixture";
import { cellSymbol, modeLevels, type StateRoles, savedCellText, toggleCell } from "./matrix";

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
  const wheel: StateRoles = { off_state: "off", main_state: "on" };
  it("停止なら主な状態へ、それ以外なら停止の状態へ", () => {
    expect(toggleCell(wheel, { state: "off", duty: {} })).toEqual({ state: "on", duty: {} });
    expect(toggleCell(wheel, null)).toEqual({ state: "on", duty: {} });
    expect(toggleCell(wheel, { state: "on", duty: {} })).toEqual({ state: "off", duty: {} });
    expect(toggleCell(wheel, { state: "peak", duty: {} })).toEqual({ state: "off", duty: {} });
  });
  it("時間割合は持ち越さない（間欠の停止を押すとオン）", () => {
    expect(toggleCell(wheel, { state: "off", duty: { on: "0.2" } })).toEqual({
      state: "on",
      duty: {},
    });
    expect(toggleCell(wheel, { state: "on", duty: { peak: "0.1" } })).toEqual({
      state: "off",
      duty: {},
    });
  });
  it("主な状態が on でない種別は、API の主な状態へ切り替える", () => {
    const trx: StateRoles = { off_state: "off", main_state: "rx" };
    expect(toggleCell(trx, null)?.state).toBe("rx");
    expect(toggleCell(trx, { state: "rx", duty: {} })?.state).toBe("off");
  });
  it("停止の状態の名前が off でない種別は、その名前で切り替える", () => {
    const halt: StateRoles = { off_state: "halt", main_state: "run" };
    expect(toggleCell(halt, { state: "halt", duty: {} })?.state).toBe("run");
    expect(toggleCell(halt, { state: "off", duty: {} })?.state).toBe("halt");
  });
  it("停止の状態が無い種別は、押してもオンのまま。語彙が空なら変えない", () => {
    const always: StateRoles = { off_state: null, main_state: "active" };
    expect(toggleCell(always, { state: "active", duty: {} })?.state).toBe("active");
    expect(toggleCell(always, null)?.state).toBe("active");
    expect(toggleCell({ off_state: null, main_state: null }, null)).toBeNull();
  });
});

describe("セルの記号", () => {
  it("時間割合があれば間欠、停止の状態と未指定は横線", () => {
    expect(cellSymbol({ state: "on", duty: {} }, "off")).toBe("●");
    expect(cellSymbol({ state: "off", duty: { on: "0.2" } }, "off")).toBe("◐");
    expect(cellSymbol({ state: "off", duty: {} }, "off")).toBe("—");
    expect(cellSymbol(null, "off")).toBe("—");
  });
  it("停止の状態の名前が off でなければ、その名前だけが横線。停止の状態が無ければいつもオン", () => {
    expect(cellSymbol({ state: "halt", duty: {} }, "halt")).toBe("—");
    expect(cellSymbol({ state: "off", duty: {} }, "halt")).toBe("●");
    expect(cellSymbol({ state: "off", duty: {} }, null)).toBe("●");
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

describe("部品ごとの列に出すもの", () => {
  it("系の名前ではなく API の mode_levels で決まる（設計のページは system 無し）", () => {
    expect(modeLevels(fixture.defs)).toBe("power_levels");
    expect(modeLevels(fixture.defs, "power")).toBe("power_levels");
    expect(modeLevels(fixture.defs, "cdh")).toBe("states");
    const swapped = {
      ...fixture.defs,
      mode_levels: [{ system: "cdh" as const, levels: "power_levels" as const }],
    };
    expect(modeLevels(swapped, "cdh")).toBe("power_levels");
    expect(modeLevels(swapped, "power")).toBe("states");
  });
});
