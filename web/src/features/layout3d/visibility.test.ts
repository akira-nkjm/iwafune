import { describe, expect, it } from "vitest";
import { isIsolated, isolate, toggleHidden, visibilityOf } from "./visibility";

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

describe("この部品だけ表示", () => {
  const placed = ["A", "B", "C"];
  it("組の外を隠し、組だけが見えているときに押すとすべて表示に戻す", () => {
    const once = isolate(new Set(), placed, ["A"]);
    expect([...once].sort()).toEqual(["B", "C"]);
    expect(isIsolated(once, placed, ["A"])).toBe(true);
    expect(isolate(once, placed, ["A"]).size).toBe(0);
  });
  it("別の部品が見えていれば分離表示ではない（押すとその組に切り替える）", () => {
    expect(isIsolated(new Set(["C"]), placed, ["A"])).toBe(false);
    expect([...isolate(new Set(["B", "C"]), placed, ["B"])].sort()).toEqual(["A", "C"]);
  });
});
