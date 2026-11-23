import { describe, expect, it } from "vitest";
import { emptyPhaseText, phaseCommand } from "./phase-form";

describe("phaseCommand", () => {
  it("ID が無ければ作らない", () => {
    expect(phaseCommand(emptyPhaseText(), true)).toBeNull();
  });

  it("打った文字列のまま、追加と編集で別のコマンドにする", () => {
    const text = { id: "LEOP", order: "1", duration: "30 d", modes: "SAFE,NOMINAL" };
    expect(phaseCommand(text, true)).toEqual({ command: "phase_add", ...text });
    expect(phaseCommand(text, false)).toEqual({ command: "phase_set", ...text });
  });
});
