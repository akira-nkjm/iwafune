import { describe, expect, it } from "vitest";
import { casePhaseCommand } from "./case-phase";

describe("casePhaseCommand", () => {
  it("フェーズを書く", () => {
    expect(casePhaseCommand("hot", "LEOP")).toEqual({
      command: "case_phase",
      case: "hot",
      phase: "LEOP",
    });
  });

  it("空のフェーズは外す（null を送る）", () => {
    expect(casePhaseCommand("hot", "")).toEqual({
      command: "case_phase",
      case: "hot",
      phase: null,
    });
  });
});
