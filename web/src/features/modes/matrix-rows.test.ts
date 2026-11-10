import { describe, expect, it } from "vitest";
import type { DesignView, ModeLoadOutput, PartRow } from "@/api/types";
import { modesJa } from "@/i18n/modes.ja";
import { fixture, qv } from "@/test/fixture";
import type { ModeDraft } from "./draft";
import { unassignedCells } from "./matrix";
import { matrixGroups } from "./matrix-rows";

/** 送受信機にも状態ごとの電力を持たせた設計（PCDU は電力が無いので行列に出ない） */
function design(): DesignView {
  const parts = fixture.parts.map((p): PartRow => {
    const base = p.values.tx_power;
    if (p.id !== "TRX" || !base) return p;
    const power = {
      type: "by_state" as const,
      states: { off: qv(0, "W", "Power"), rx: qv(2, "W", "Power"), tx: qv(5, "W", "Power") },
    };
    return {
      ...p,
      count: 2,
      values: { ...p.values, power: { ...base, attr: "power", value: power } },
    };
  });
  return { ...fixture, parts };
}

function load(mean: number): ModeLoadOutput {
  const part = {
    part: "TRX",
    label: "TRX",
    group: "COM",
    states: [],
    mean,
    mean_with_growth: mean,
    growth_margin: 0,
  };
  const mode = {
    id: "NOMINAL",
    label: "定常",
    total: mean,
    total_with_growth: mean,
    peak_with_growth: mean,
    groups: [],
    parts: [part],
    unassigned: [],
    missing_power: [],
    checks: [],
  };
  return { modes: [mode] };
}

describe("matrixGroups", () => {
  it("電力を使う部品だけを、サブシステムごとに ID の順で並べ、セルの位置の行を通しで振る", () => {
    const groups = matrixGroups(design(), new Map(), null, modesJa.modes);
    expect(groups.map((g) => [g.group, g.label, g.rows.map((r) => r.part)])).toEqual([
      ["AOCS", "姿勢制御系", ["RW_X"]],
      ["COM", "通信系", ["TRX"]],
    ]);
    // 系の見出しの行をまたいでも、矢印キーで上下に動けるよう行の番号は通し
    const at = groups.flatMap((g) => g.rows.flatMap((r) => r.cells.map((c) => c.at)));
    expect(at).toEqual([
      { row: 0, column: 0 },
      { row: 0, column: 1 },
      { row: 1, column: 0 },
      { row: 1, column: 1 },
    ]);
  });

  it("行には員数・電力レベル（小さい順。API の整形）と、モードの数だけのセルを持つ", () => {
    const trx = matrixGroups(design(), new Map(), null, modesJa.modes)[1]?.rows[0];
    expect(trx?.count).toBe(2);
    expect(trx?.levels).toBe("0 W / 2 W / 5 W");
    expect(trx?.levelsTitle).toBe("off: 0 W / rx: 2 W / tx: 5 W");
    expect(trx?.cells.map((c) => c.mode)).toEqual(["NOMINAL", "SAFE"]);
  });

  it("セルだけで描けるよう、部品と状態の語彙もセルに持つ", () => {
    const trx = matrixGroups(design(), new Map(), null, modesJa.modes)[1]?.rows[0];
    expect(trx?.cells.map((c) => [c.part, c.states])).toEqual([
      ["TRX", ["off", "rx", "tx"]],
      ["TRX", ["off", "rx", "tx"]],
    ]);
  });

  it("保存済みのセルは文言と濃さ、電力は下書きを当てた解析の出力から", () => {
    const [nominal, safe] =
      matrixGroups(design(), new Map(), load(2.3), modesJa.modes)[1]?.rows[0]?.cells ?? [];
    expect(nominal).toMatchObject({
      value: { state: "rx", duty: { tx: "0.1" } },
      display: "rx（tx 10 %）",
      changed: false,
      power: 2.3,
      shade: 1,
    });
    expect(safe).toMatchObject({ value: null, display: undefined, power: null, shade: -1 });
  });

  it("下書きのセルは下書きの値で、変えた印を付ける", () => {
    const draft: ModeDraft = new Map([["SAFE", new Map([["TRX", { state: "tx", duty: {} }]])]]);
    const safe = matrixGroups(design(), draft, null, modesJa.modes)[1]?.rows[0]?.cells[1];
    expect(safe).toMatchObject({ value: { state: "tx", duty: {} }, changed: true, shade: 2 });
  });
});

describe("unassignedCells", () => {
  it("部品 × モードのうち、状態の書かれていないセルを数える", () => {
    // RW_X は SAFE、TRX は SAFE が無い（PCDU は両方ある）
    expect(unassignedCells(fixture.modes)).toBe(2);
  });
});
