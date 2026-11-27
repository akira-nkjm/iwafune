import { describe, expect, it } from "vitest";
import { checkScenarios, collectShots, parseArgs } from "./capture-shots.ts";
import type { Scenario } from "./shots/scenarios.ts";

describe("parseArgs", () => {
  it("fune の場所が必須", () => {
    expect(() => parseArgs([], {})).toThrow(/--fune/);
    expect(parseArgs(["--fune", "/x/fune"], {}).langs).toEqual(["en", "ja"]);
    expect(parseArgs(["--lang", "ja", "--only", "a,b"], { FUNE_BIN: "/x/fune" })).toMatchObject({
      langs: ["ja"],
      only: ["a", "b"],
    });
  });
});

describe("collectShots / checkScenarios", () => {
  it("docs の目印は、名前付きを別の写真として集める", () => {
    const shots = collectShots(new URL("../../docs", import.meta.url).pathname);
    const ids = shots.map((s) => s.id);
    expect(new Set(ids).size).toBe(ids.length);
    expect(ids).toContain("home");
  });
  it("名前付きでシナリオが無い写真と、目印の無いシナリオを見つける", () => {
    const shots = [
      { id: "home", route: "/", description: "d" },
      { id: "home--x", route: "/", name: "x", description: "d" },
    ];
    const none: Record<string, Scenario> = {};
    expect(checkScenarios(shots, none)).toEqual(["named shot home--x has no scenario"]);
    expect(checkScenarios(shots, { "home--x": {}, gone: {} })).toEqual([
      "scenario gone has no marker in docs/",
    ]);
  });
});
