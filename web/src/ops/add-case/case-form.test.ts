import { describe, expect, it } from "vitest";
import type { DesignView } from "@/api/types";
import { fixture } from "@/test/fixture";
import { caseChoices, caseSetCommand, emptyCase } from "./case-form";

describe("ケースの欄 → fune case set の引数", () => {
  it("ID が空なら送らず、ほかの欄は空でもよい（書かなかった欄は書かれない）", () => {
    expect(caseSetCommand(emptyCase())).toBeNull();
    expect(caseSetCommand({ ...emptyCase(), id: " " })).toBeNull();
    expect(caseSetCommand({ ...emptyCase(), id: "HOT" })).toEqual({
      command: "case_set",
      ...emptyCase("HOT"),
    });
  });

  it("量・周回数・時刻は読まずにそのまま渡す", () => {
    const c = { ...emptyCase("HOT"), beta: "60 deg", orbits: "5", epoch: "2031-05-02T00:00:00Z" };
    expect(caseSetCommand(c)).toEqual({ command: "case_set", ...c });
  });

  it("モード・フェーズ・熱の環境条件の選択肢は設計と API の add_choices から作る", () => {
    const design = {
      ...fixture,
      modes: { ...fixture.modes, modes: [{ id: "SAFE", label: "Safe" }] },
      phases: [{ id: "LEOP", label: "Commissioning" }],
      add_choices: { transceivers: [], bands: [], thermal_cases: ["cold", "hot"] },
    } as unknown as DesignView;
    const choices = caseChoices(design);
    expect(choices.modes.map((o) => o.value)).toEqual(["SAFE"]);
    expect(choices.phases.map((o) => o.value)).toEqual(["LEOP"]);
    expect(choices.thermalCases.map((o) => o.value)).toEqual(["cold", "hot"]);
  });
});
