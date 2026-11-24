import { describe, expect, it } from "vitest";
import { caseEpochCommand } from "./case-epoch";

describe("caseEpochCommand", () => {
  it("打った時刻をそのまま送る（読むのは service）", () => {
    expect(caseEpochCommand("hot", " 2031-05-02T00:00:00Z ")).toEqual({
      command: "case_epoch",
      case: "hot",
      epoch: "2031-05-02T00:00:00Z",
    });
  });

  it("空は外す（null を送る）", () => {
    expect(caseEpochCommand("hot", "  ")).toEqual({
      command: "case_epoch",
      case: "hot",
      epoch: null,
    });
  });
});
