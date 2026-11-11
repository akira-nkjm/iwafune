import { describe, expect, it } from "vitest";
import { systemTone } from "./systems";

describe("系の色", () => {
  it("API の系を使い、未所属は中立色、payload にも色を持つ", () => {
    expect(systemTone("aocs")).toBe("blue");
    expect(systemTone("power")).toBe("green");
    expect(systemTone("comm")).toBe("gold");
    expect(systemTone("payload")).toBe("rose");
    expect(systemTone(null)).toBe("gray");
    expect(systemTone(undefined)).toBe("gray");
  });
});
