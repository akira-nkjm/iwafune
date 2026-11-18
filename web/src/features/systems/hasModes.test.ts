import { describe, expect, it } from "vitest";
import { fixture } from "@/test/fixture";
import { hasModes } from "./SystemTabs";

describe("hasModes", () => {
  it("モードの見方を持つ系は API（DefsView.mode_systems）が決める", () => {
    expect(hasModes("power", fixture.defs)).toBe(true);
    expect(hasModes("comm", fixture.defs)).toBe(false);
    expect(hasModes("comm", { ...fixture.defs, mode_systems: ["comm"] })).toBe(true);
    expect(hasModes("power", undefined)).toBe(false);
  });
});
