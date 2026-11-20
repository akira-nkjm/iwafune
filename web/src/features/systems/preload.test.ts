import { describe, expect, it } from "vitest";
import { hasExtraTab } from "./preload";

describe("追加のタブ", () => {
  it("解析結果のタブは熱だけ", () => {
    expect(hasExtraTab("thermal", "analyses")).toBe(true);
    expect(hasExtraTab("power", "analyses")).toBe(false);
  });
  it("熱画像は熱だけ", () => {
    expect(hasExtraTab("thermal", "heat")).toBe(true);
    expect(hasExtraTab("power", "heat")).toBe(false);
    expect(hasExtraTab("structure", "heat")).toBe(false);
    expect(hasExtraTab("thermal", "modes")).toBe(false);
  });
  it("マヌーバの定義は推進だけ", () => {
    expect(hasExtraTab("propulsion", "maneuvers")).toBe(true);
    expect(hasExtraTab("thermal", "maneuvers")).toBe(false);
    expect(hasExtraTab("propulsion", "heat")).toBe(false);
  });
});
