import { describe, expect, it, vi } from "vitest";
import { clampTreeWidth, readTreeWidth, TREE_WIDTH, writeTreeWidth } from "./tree-width";

function memoryStorage(value: string | null = null) {
  let saved = value;
  return {
    getItem: vi.fn((_key: string) => saved),
    setItem: vi.fn((_key: string, next: string) => {
      saved = next;
    }),
  };
}

describe("tree width", () => {
  it("clamps drags and saved widths to the explorer range", () => {
    expect(clampTreeWidth(350)).toBe(350);
    expect(clampTreeWidth(20)).toBe(TREE_WIDTH.min);
    expect(clampTreeWidth(2000)).toBe(TREE_WIDTH.max);
    expect(clampTreeWidth(Number.NaN)).toBe(TREE_WIDTH.initial);
    expect(clampTreeWidth(Number.POSITIVE_INFINITY)).toBe(TREE_WIDTH.initial);
  });
  it.each([null, "", "  ", "not a width", "NaN", "Infinity"])(
    "uses the default for missing or malformed storage: %s",
    (value) => {
      expect(readTreeWidth(() => memoryStorage(value))).toBe(TREE_WIDTH.initial);
    },
  );
  it("restores and clamps numeric saved values", () => {
    expect(readTreeWidth(() => memoryStorage("370"))).toBe(370);
    expect(readTreeWidth(() => memoryStorage("-1"))).toBe(TREE_WIDTH.min);
    expect(readTreeWidth(() => memoryStorage("999"))).toBe(TREE_WIDTH.max);
  });
  it("round trips both pages through the same storage key", () => {
    const storage = memoryStorage();
    writeTreeWidth(360, () => storage);
    expect(readTreeWidth(() => storage)).toBe(360);
    expect(storage.setItem.mock.calls[0]?.[0]).toBe(storage.getItem.mock.calls[0]?.[0]);
    writeTreeWidth(900, () => storage);
    expect(readTreeWidth(() => storage)).toBe(TREE_WIDTH.max);
  });
  it("tolerates unavailable window and a denied localStorage getter", () => {
    expect(readTreeWidth()).toBe(TREE_WIDTH.initial);
    expect(() => writeTreeWidth(350)).not.toThrow();
    const denied = () => {
      throw new Error("storage blocked");
    };
    expect(readTreeWidth(denied)).toBe(TREE_WIDTH.initial);
    expect(() => writeTreeWidth(350, denied)).not.toThrow();
  });
  it("tolerates reads and quota-limited writes throwing", () => {
    const storage = {
      getItem: () => {
        throw new Error("read denied");
      },
      setItem: () => {
        throw new Error("quota exceeded");
      },
    };
    expect(readTreeWidth(() => storage)).toBe(TREE_WIDTH.initial);
    expect(() => writeTreeWidth(350, () => storage)).not.toThrow();
  });
});
