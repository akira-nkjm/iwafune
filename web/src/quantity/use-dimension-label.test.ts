import { afterEach, describe, expect, it } from "vitest";
import type { DimensionInfo } from "@/api/types";
import { setLang } from "@/i18n";
import { dimensionLabelOf } from "./use-dimension-label";

const dims: DimensionInfo[] = [
  {
    name: "AngularMomentum",
    label: "角運動量",
    label_en: "Angular momentum",
    si_unit: "kg*m^2/s",
    display_units: [],
  },
];

describe("dimensionLabelOf", () => {
  afterEach(() => setLang("ja"));

  it("現在の言語の表示名を返す", () => {
    setLang("en");
    expect(dimensionLabelOf(dims, "AngularMomentum")).toBe("Angular momentum");
    setLang("ja");
    expect(dimensionLabelOf(dims, "AngularMomentum")).toBe("角運動量");
  });

  it("一覧に無い名前は次元名のまま返す", () => {
    expect(dimensionLabelOf(dims, "Fraction")).toBe("Fraction");
    expect(dimensionLabelOf([], "AngularMomentum")).toBe("AngularMomentum");
  });
});
