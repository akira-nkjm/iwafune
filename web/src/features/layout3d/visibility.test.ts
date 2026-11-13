import { describe, expect, it } from "vitest";
import { toggleHidden, visibilityOf } from "./visibility";

describe("visibilityOf", () => {
  it("隠した数で、見える・隠れた・一部だけを返す", () => {
    const hidden = new Set(["A", "B"]);
    expect(visibilityOf(hidden, ["C"])).toBe("shown");
    expect(visibilityOf(hidden, ["A", "B"])).toBe("hidden");
    expect(visibilityOf(hidden, ["A", "C"])).toBe("mixed");
    expect(visibilityOf(hidden, [])).toBe("shown");
  });
});

describe("toggleHidden", () => {
  it("1 つでも見えていればまとめて隠し、すべて隠れていればまとめて出す。ほかの ID は保つ", () => {
    const once = toggleHidden(new Set(["X", "A"]), ["A", "B"]);
    expect([...once].sort()).toEqual(["A", "B", "X"]);
    const twice = toggleHidden(once, ["A", "B"]);
    expect([...twice]).toEqual(["X"]);
  });

  it("元の集合は変えない", () => {
    const hidden = new Set(["A"]);
    toggleHidden(hidden, ["B"]);
    expect([...hidden]).toEqual(["A"]);
  });
});
