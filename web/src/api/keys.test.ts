import { describe, expect, it } from "vitest";
import { invalidatedByOps, keys } from "./keys";

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
});
