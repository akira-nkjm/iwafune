import { describe, expect, it } from "vitest";
import { emptyRequirementText, requirementCommand } from "./requirement-form";

describe("要求の欄 → fune requirement add|set の引数", () => {
  it("ID が空なら送らず、打った文字列は読まずにそのまま渡す", () => {
    expect(requirementCommand(emptyRequirementText(), true)).toBeNull();
    const text = {
      ...emptyRequirementText(),
      id: "R_MASS",
      statement: "Mass",
      quantity: "Mass",
      value: "12 kg",
      cases: "a,b",
    };
    expect(requirementCommand(text, true)).toEqual({ command: "requirement_add", ...text });
    expect(requirementCommand(text, false)?.command).toBe("requirement_set");
  });
});
