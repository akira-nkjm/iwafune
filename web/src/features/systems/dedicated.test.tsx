import { describe, expect, it } from "vitest";
import { fixture } from "@/test/fixture";
import {
  hasExtraTab,
  hasModes,
  resolveTab,
  SYSTEM_REGISTRY,
  SYSTEM_TAB_ORDER,
  systemChunks,
  systemExtras,
  systemRegistration,
  systemTabs,
} from "./dedicated";
import { tabFillsBody } from "./SystemTabs";

describe("系ごとの登録表", () => {
  it("登録した系はすべて API の系の一覧にある（登録表のキーは未知の系にならない）", () => {
    for (const system of Object.keys(SYSTEM_REGISTRY)) {
      expect(fixture.defs.systems as string[]).toContain(system);
    }
  });

  it("専用のモードの表示を持つ系は、モードの表が本文の高さを使わない（登録表で決まる）", () => {
    expect(systemRegistration("power")?.modes).toBeDefined();
    expect(systemRegistration("power")?.series).toBeDefined();
    expect(systemRegistration("aocs")?.modes).toBeDefined();
    expect(systemRegistration("aocs")?.series).toBeUndefined();
    expect(systemRegistration("cdh")).toBeUndefined();
    // 通信は専用のタブを持たず、ページの右上に回線を足す操作だけを登録する
    expect(systemRegistration("comm")?.actions).toBeDefined();
    expect(systemRegistration("comm")?.modes).toBeUndefined();
    expect(tabFillsBody("power", "modes", fixture.defs)).toBe(false);
    expect(tabFillsBody("aocs", "modes", fixture.defs)).toBe(false);
    expect(tabFillsBody("thermal", "modes", fixture.defs)).toBe(true);
    expect(tabFillsBody("comm", "modes", fixture.defs)).toBe(false);
    expect(tabFillsBody("thermal", "results", fixture.defs)).toBe(false);
  });

  it("登録した系の専用の表示は、設計が無いときも例外を出さない", () => {
    for (const system of Object.keys(SYSTEM_REGISTRY) as (keyof typeof SYSTEM_REGISTRY)[]) {
      const views = systemRegistration(system);
      expect(() => views?.modes?.({ design: undefined, kase: null })).not.toThrow();
      expect(() => views?.actions?.({ design: undefined, kase: null })).not.toThrow();
    }
  });

  it("追加の表示は設計が届いてから作る。構造の概要には何も足さない", () => {
    expect(systemExtras("structure", fixture, [])).toEqual({});
    expect(systemExtras("thermal", undefined, [])).toEqual({});
  });

  it("大きなチャンクは熱だけが持つ", () => {
    expect(systemChunks("thermal")).toHaveLength(1);
    expect(systemChunks("power")).toHaveLength(0);
  });
});

describe("系のページのタブ", () => {
  it("解析結果・熱画像・結合と表面は熱だけ、マヌーバの定義は推進だけ", () => {
    expect(hasExtraTab("thermal", "analyses")).toBe(true);
    expect(hasExtraTab("power", "analyses")).toBe(false);
    expect(hasExtraTab("thermal", "heat")).toBe(true);
    expect(hasExtraTab("power", "heat")).toBe(false);
    expect(hasExtraTab("structure", "heat")).toBe(false);
    expect(hasExtraTab("thermal", "modes")).toBe(false);
    expect(hasExtraTab("propulsion", "maneuvers")).toBe(true);
    expect(hasExtraTab("thermal", "maneuvers")).toBe(false);
    expect(hasExtraTab("propulsion", "heat")).toBe(false);
  });

  it("モードのタブは API が返した系だけ", () => {
    expect(hasModes("power", fixture.defs)).toBe(true);
    expect(hasModes("comm", fixture.defs)).toBe(false);
    expect(hasModes("comm", { ...fixture.defs, mode_systems: ["comm"] })).toBe(true);
    expect(hasModes("power", undefined)).toBe(false);
  });

  it("タブは決まった並びで、系ごとに出し分ける（並びは変わらない）", () => {
    expect(systemTabs("thermal", fixture.defs)).toEqual([
      "results",
      "heat",
      "couplings",
      "analyses",
      "modes",
      "series",
      "settings",
    ]);
    expect(systemTabs("propulsion", fixture.defs)).toEqual([
      "results",
      "maneuvers",
      "series",
      "settings",
    ]);
    expect(systemTabs("power", fixture.defs)).toEqual(["results", "modes", "series", "settings"]);
    expect(systemTabs("comm", fixture.defs)).toEqual(["results", "series", "settings"]);
    // 設計を読むまではモードのタブを出さない
    expect(systemTabs("power", undefined)).toEqual(["results", "series", "settings"]);
    for (const system of fixture.defs.systems) {
      const order = systemTabs(system, fixture.defs).map((t) => SYSTEM_TAB_ORDER.indexOf(t));
      expect(order).toEqual([...order].sort((a, b) => a - b));
    }
  });

  it("URL のタブが系に無ければ結果のタブに戻す", () => {
    expect(resolveTab("thermal", "heat", fixture.defs)).toBe("heat");
    expect(resolveTab("power", "heat", fixture.defs)).toBe("results");
    expect(resolveTab("power", "nonsense", fixture.defs)).toBe("results");
    expect(resolveTab("power", undefined, fixture.defs)).toBe("results");
  });
});
