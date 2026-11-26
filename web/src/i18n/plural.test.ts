import { describe, expect, it } from "vitest";
import { dictionaries } from "./index";
import { plural, verb } from "./plural";

describe("英語の個数の文", () => {
  it("1 は単数、それ以外は複数", () => {
    expect(plural(1, "cell")).toBe("1 cell");
    expect(plural(0, "cell")).toBe("0 cells");
    expect(plural(2, "category", "categories")).toBe("2 categories");
    expect(verb(1, "needs", "need")).toBe("needs");
  });

  it("辞書の文が 1 件で単数になる", () => {
    const t = dictionaries.en;
    expect(t.modes.changed(1)).toBe("1 cell changed");
    expect(t.catalog.shared.removeConfirm).toBe("Remove");
    expect(t.overview.nodesToCheck(1)).toBe("1 node needs attention");
    expect(t.overview.nodesToCheck(3)).toBe("3 nodes need attention");
  });
});
