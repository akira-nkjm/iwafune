import { describe, expect, it } from "vitest";
import { invalidatedByOps, keys, refersToRetired, retiredBy } from "./keys";

const startsWith = (key: readonly unknown[], prefix: readonly unknown[]) =>
  prefix.every((p, i) => key[i] === p);

describe("query keys", () => {
  it("編集の後に、設計・部品・定義・診断・解析・バジェットがすべて無効化される", () => {
    const affected = [
      keys.project,
      keys.design,
      keys.part("RW_X"),
      keys.part("RW_X", '[{"op":"create_attribute"}]'),
      keys.defs,
      keys.diagnostics,
      keys.history,
      keys.analysis("power.mode_load", null),
      keys.analyzedModes("power", null, "[]"),
      keys.analyzedModes("power", "hot_imaging", "[]"),
      keys.systemModes("power", "[]", "totals"),
      keys.budget("mass"),
    ];
    for (const k of affected) {
      expect(
        invalidatedByOps.some((p) => startsWith(k, p)),
        JSON.stringify(k),
      ).toBe(true);
    }
  });

  it("単位の一覧・解析の一覧・外部データの状態は編集で変わらない", () => {
    for (const k of [keys.units(null), keys.analyses, keys.parse("1 kg", null), keys.dataStatus]) {
      expect(invalidatedByOps.some((p) => startsWith(k, p))).toBe(false);
    }
  });

  it("解析の結果のキーはどれも analysisResults で始まる（天体の暦を読み込んだら読み直す）", () => {
    for (const k of [
      keys.analysis("comm.visibility", null),
      keys.analysis("x", "hot", "[]"),
      keys.analyzedModes("power", "hot", "[]"),
    ]) {
      expect(startsWith(k, keys.analysisResults)).toBe(true);
    }
    expect(startsWith(keys.analyses, keys.analysisResults)).toBe(false);
  });

  it("消す・名前を変える Op の対象（変わる前の ID）を、キーに ID を持つ種類だけ集める", () => {
    const r = retiredBy([
      { op: "remove_part", id: "A" },
      { op: "rename_part", id: "B", to: "B2" },
      { op: "remove_configuration", id: "C" },
      { op: "rename_configuration", id: "D", to: "D2" },
      { op: "remove_case", id: "E" },
      { op: "rename_case", id: "F", to: "F2" },
      { op: "remove_mode", id: "G" },
    ]);
    expect([...r.parts]).toEqual(["A", "B"]);
    expect([...r.configurations]).toEqual(["C", "D"]);
    expect([...r.cases]).toEqual(["E", "F"]);
  });

  it("キーの作り方ごとに、消えた対象を指すかを読める（keys の形と合っている）", () => {
    const r = retiredBy([
      { op: "remove_part", id: "P" },
      { op: "remove_configuration", id: "CFG" },
      { op: "remove_case", id: "CASE" },
    ]);
    const stale = [
      keys.parts("CFG"),
      keys.part("P"),
      keys.part("P", '[{"op":"x"}]'),
      keys.part("Q", "", "CFG"),
      keys.analysis("structure.mass_properties", null, "", "CFG"),
      keys.analysis("structure.mass_properties", null, '[{"op":"x"}]', "CFG"),
      keys.analysisDetail("structure.mass_properties", null, "CFG"),
      keys.analysis("power.mode_load", "CASE"),
      keys.analysis("power.mode_load", "CASE", '[{"op":"x"}]', "OTHER"),
      keys.analysisDetail("power.mode_load", "CASE"),
      keys.analyzedModes("power", "CASE", "[]"),
      keys.thermalTables("CASE", "[]"),
    ];
    const live = [
      keys.parts(null),
      keys.parts("OTHER"),
      keys.part("Q"),
      keys.part("Q", "", "OTHER"),
      keys.analysis("structure.mass_properties", null),
      keys.analysis("structure.mass_properties", null, "", "OTHER"),
      keys.analysis("power.mode_load", "other"),
      keys.analyzedModes("power", null, "[]"),
      keys.thermalTables("other", "[]"),
      keys.design,
      keys.defs,
      keys.budget("mass"),
    ];
    for (const k of stale) expect(refersToRetired(k, r), JSON.stringify(k)).toBe(true);
    for (const k of live) expect(refersToRetired(k, r), JSON.stringify(k)).toBe(false);
  });
});
