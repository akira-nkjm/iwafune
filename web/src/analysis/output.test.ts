import { describe, expect, it } from "vitest";
import {
  collectScalars,
  collectSeries,
  collectTables,
  dimOf,
  EACH,
  groupScalars,
  jsonPointer,
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

describe("部品の干渉（structure.interference）の出力の形", () => {
  const interference = {
    tolerance: 0.0001,
    parts_judged: 3,
    pairs_judged: 3,
    pairs_in_mount_chain: 0,
    overlaps: [
      { first: "A", second: "B", depth: 0.04, separation_axis: { x: 0, y: 0, z: -1 } },
      { first: "A", second: "C", depth: 0.001, separation_axis: { x: 1, y: 0, z: 0 } },
    ],
    min_clearance: null,
    skipped: [{ id: "HTR", reason: "no_size" }],
    repeated: [],
    checks: [{ key: "overlap:A:B", label: "Boxes", verdict: "warning", message: "" }],
  };

  it("重なっている組と判定しなかった部品は表になり、深さは次元つきの数、向きは 1 つのセルになる", () => {
    const tables = collectTables(interference);
    expect(tables.map((t) => t.key)).toEqual(["overlaps", "skipped"]);
    expect(tableColumns(tables[0]?.rows ?? [])).toEqual([
      "first",
      "second",
      "depth",
      "separation_axis",
    ]);
    expect(collectScalars(interference).map((s) => s.key)).toEqual([
      "tolerance",
      "parts_judged",
      "pairs_judged",
      "pairs_in_mount_chain",
      "min_clearance",
    ]);
  });
});

describe("噴流・視野の遮り（structure.cone_clearance）の出力の形", () => {
  const cone = {
    range: 0.6,
    range_is_default: true,
    parts_with_box: 36,
    cones_judged: 1,
    cones_obstructed: 1,
    cones: [
      {
        id: "STT",
        kind: "field_of_view",
        half_angle: 0.17,
        cone_axis: { x: 0, y: -1, z: 0 },
        apex: { x: 0, y: -0.11, z: -0.03 },
        range: 0.6,
        parts_judged: 33,
        parts_in_mount_chain: 2,
        obstructions: [{ part: "SP_MY", min_angle: 0, min_distance: 0.0007 }],
      },
    ],
    skipped_carriers: [{ id: "EPT", kind: "plume", reason: "no_half_angle" }],
    skipped: [],
    checks: [{ key: "obstruction:STT:SP_MY", label: "x", verdict: "warning", message: "" }],
  };

  it("円錐と判定しなかった持ち主は表になり、数は単独の値になる", () => {
    const tables = collectTables(cone);
    expect(tables.map((t) => t.key)).toEqual(["cones", "skipped_carriers"]);
    expect(collectScalars(cone).map((s) => s.key)).toEqual([
      "range",
      "range_is_default",
      "parts_with_box",
      "cones_judged",
      "cones_obstructed",
    ]);
  });
});
