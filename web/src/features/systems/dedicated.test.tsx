import { describe, expect, it } from "vitest";
import { fixture } from "@/test/fixture";
import { DEDICATED_VIEWS, dedicatedViews } from "./dedicated";
import { tabFillsBody } from "./SystemTabs";

describe("専用の表示の登録表", () => {
  it("専用のモードの表示を持つ系は、モードの表が本文の高さを使わない（登録表で決まる）", () => {
    expect(dedicatedViews("power")?.modes).toBeDefined();
    expect(dedicatedViews("power")?.series).toBeDefined();
    expect(dedicatedViews("aocs")?.modes).toBeDefined();
    expect(dedicatedViews("aocs")?.series).toBeUndefined();
    expect(dedicatedViews("thermal")).toBeUndefined();
    // 通信は専用のタブを持たず、ページの右上に回線を足す操作だけを登録する
    expect(dedicatedViews("comm")?.actions).toBeDefined();
    expect(dedicatedViews("comm")?.modes).toBeUndefined();
    expect(tabFillsBody("power", "modes", fixture.defs)).toBe(false);
    expect(tabFillsBody("aocs", "modes", fixture.defs)).toBe(false);
    expect(tabFillsBody("thermal", "modes", fixture.defs)).toBe(true);
    expect(tabFillsBody("comm", "modes", fixture.defs)).toBe(false);
    expect(tabFillsBody("thermal", "results", fixture.defs)).toBe(false);
  });

  it("登録した系のモードの表示は、設計が無いときも例外を出さない", () => {
    for (const system of Object.keys(DEDICATED_VIEWS) as (keyof typeof DEDICATED_VIEWS)[]) {
      const views = dedicatedViews(system);
      expect(() => views?.modes?.({ design: undefined, kase: null })).not.toThrow();
      expect(() => views?.actions?.({ design: undefined, kase: null })).not.toThrow();
    }
  });
});
