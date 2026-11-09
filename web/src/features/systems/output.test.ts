import { describe, expect, it } from "vitest";
import {
  collectScalars,
  collectSeries,
  collectTables,
  dimOf,
  EACH,
  groupScalars,
  jsonPointer,
  pickNumbers,
  tableColumns,
} from "./output";

const out = {
  life: "eol",
  period: 5600,
  summary: { total_mass: 8.2, center_of_mass: { x: 0, y: 0.01, z: 0.1 } },
  series: {
    t: [0, 60],
    channels: [
      {
        key: "soc",
        label: "SOC",
        dimension: "Fraction",
        display_unit: "%",
        display_scale: 100,
        values: [1, 0.9],
      },
    ],
  },
  cases: [
    { id: "worst", max_dod: 0.21, orbits: [{ index: 0, margin: 12 }] },
    { id: "best", max_dod: 0.12, orbits: [] },
  ],
  checks: [{ key: "dod", label: "DoD", verdict: "ok", message: "21% ≤ 30%" }],
};

describe("解析の出力の読み取り", () => {
  it("スカラー（入れ子のオブジェクトの中も）を集め、checks と時系列は除く", () => {
    const scalars = collectScalars(out);
    expect(scalars.map((s) => s.path)).toEqual([
      "life",
      "period",
      "summary.total_mass",
      "summary.center_of_mass",
    ]);
    expect(scalars[2]).toMatchObject({
      at: ["summary", "total_mass"],
      pointer: "/summary/total_mass",
    });
  });
  it("Series を集める", () => {
    expect(collectSeries(out).map((s) => s.path)).toEqual(["series"]);
  });
  it("オブジェクトの配列は表になり、入れ子の配列は列にしない", () => {
    const t = collectTables(out);
    expect(t.map((x) => x.path)).toEqual(["cases"]);
    expect(t[0]!.at).toEqual(["cases"]);
    expect(tableColumns(t[0]!.rows)).toEqual(["id", "max_dod"]);
  });
  it("欄の指し方で数を引く（EACH で配列を展開）", () => {
    expect(pickNumbers(out, ["cases", EACH, "max_dod"])).toEqual([0.21, 0.12]);
    expect(pickNumbers(out, ["summary", "total_mass"])).toEqual([8.2]);
    expect(pickNumbers(out, ["nope", "x"])).toEqual([]);
  });
});

describe("スカラーの区分", () => {
  it("入れ子のオブジェクトごとに分け、最上位を先にする", () => {
    const g = groupScalars(collectScalars({ mtq: { time: 10 }, period: 5, rcs: { time: 2 } }));
    expect(g.map(([k, v]) => [k, v.map((s) => s.key)])).toEqual([
      ["", ["period"]],
      ["mtq", ["time"]],
      ["rcs", ["time"]],
    ]);
  });
});

describe("JSON Pointer", () => {
  it("区間の列から作り、RFC 6901 のとおり ~ と / を逃がす", () => {
    expect(jsonPointer(["cases", 0, "max_dod"])).toBe("/cases/0/max_dod");
    expect(jsonPointer(["a/b", "c~d"])).toBe("/a~1b/c~0d");
    expect(jsonPointer([])).toBe("");
  });
  it("キーに . や / を含んでも分解しない", () => {
    const found = collectScalars({ "local/rw.x": { "p.q": 1 } });
    expect(found[0]).toMatchObject({ at: ["local/rw.x", "p.q"], pointer: "/local~1rw.x/p.q" });
  });
});

describe("dims（次元）の引き当て", () => {
  it("欄の指し方（EACH で配列）に当たる位置の JSON Pointer で次元を引く", () => {
    const result = {
      output: { cases: [{ max_dod: 0.2 }, { load_sunlit: 3 }], period: 5600, "a/b": { c: 1 } },
      dims: {
        "/cases/0/max_dod": "Fraction",
        "/cases/1/load_sunlit": "Power",
        "/period": "Time",
        "/a~1b/c": "Length",
      },
    };
    expect(dimOf(result, ["cases", EACH, "load_sunlit"])).toBe("Power");
    expect(dimOf(result, ["period"])).toBe("Time");
    expect(dimOf(result, ["a/b", "c"])).toBe("Length");
    expect(dimOf(result, ["cases", EACH, "nope"])).toBeNull();
    expect(dimOf(undefined, ["period"])).toBeNull();
  });
});
