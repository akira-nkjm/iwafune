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
      keys.defs,
      keys.diagnostics,
      keys.analysis("power.mode_load", null),
      keys.budget("mass"),
    ];
    for (const k of affected) {
      expect(
        invalidatedByOps.some((p) => startsWith(k, p)),
        JSON.stringify(k),
      ).toBe(true);
    }
  });

  it("単位の一覧と解析の一覧は編集で変わらない", () => {
    for (const k of [keys.units(null), keys.analyses, keys.parse("1 kg", null)]) {
      expect(invalidatedByOps.some((p) => startsWith(k, p))).toBe(false);
    }
  });
});
