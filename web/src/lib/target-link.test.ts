import { describe, expect, it } from "vitest";
import type { DesignTarget } from "@/api/types";
import { targetLink } from "./target-link";

const parts = new Set(["RW_X"]);

describe("targetLink", () => {
  it("マヌーバの対象は、推進のページのマヌーバのタブへ", () => {
    expect(targetLink({ kind: "maneuver", id: "burn-1", field: null }, parts)).toBe(
      "/systems/propulsion/maneuvers?maneuver=burn-1",
    );
    expect(targetLink({ kind: "maneuvers" }, parts)).toBe("/systems/propulsion/maneuvers");
  });

  it("部品の対象は、設計にある部品のページへ", () => {
    const attr: DesignTarget = {
      kind: "param",
      target: "attr",
      part: "RW_X",
      attr: "mass",
      state: null,
    };
    expect(targetLink(attr, parts)).toBe("/design/components/RW_X");
    expect(targetLink({ kind: "mount", part: "RW_X" }, parts)).toBe("/design/components/RW_X");
    expect(targetLink({ kind: "part", id: "GONE" }, parts)).toBeNull();
  });

  it("どれかの部品・設定・モード・ケースは、部品の ID を作らずにそれぞれのページへ", () => {
    expect(targetLink({ kind: "any_part", roles: [], attr: "power" }, parts)).toBe(
      "/design/components",
    );
    const setting: DesignTarget = {
      kind: "param",
      target: "setting",
      scope: "analysis",
      segments: ["power", "dod_max"],
    };
    expect(targetLink(setting, parts)).toBe("/project/settings");
    expect(targetLink({ kind: "mode", id: "NOMINAL", part: "TRX" }, parts)).toBe("/design/modes");
    expect(targetLink({ kind: "case", id: "HOT", field: "beta" }, parts)).toBe("/conditions/cases");
    expect(targetLink({ kind: "source", id: "ds_vendor" }, parts)).toBe("/project/sources");
  });

  it("部品の ID も URL に入れるときに符号化する", () => {
    expect(targetLink({ kind: "part", id: "A B" }, new Set(["A B"]))).toBe(
      "/design/components/A%20B",
    );
  });

  it("カタログ品の ID は URL に入れるときだけ符号化し、対象が無ければ null", () => {
    expect(targetLink({ kind: "catalog", id: "local/rw", attr: null }, parts)).toBe(
      "/design/catalog/local%2Frw",
    );
    expect(targetLink(null, parts)).toBeNull();
    expect(targetLink({ kind: "analysis", id: "cdh.data_budget" }, parts)).toBeNull();
  });
});
