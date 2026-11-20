import { describe, expect, it } from "vitest";
import { hasExtraTab } from "./preload";

describe("追加のタブ", () => {
  it("熱画像は熱だけ", () => {
    expect(hasExtraTab("thermal", "heat")).toBe(true);
    expect(hasExtraTab("power", "heat")).toBe(false);
    expect(hasExtraTab("structure", "heat")).toBe(false);
    expect(hasExtraTab("thermal", "modes")).toBe(false);
  });
});
