import { describe, expect, it } from "vitest";
import { itemLabel } from "./catalog";

describe("カタログ", () => {
  it("表示名が無ければ ID を出す", () => {
    expect(itemLabel({ id: "local/rw_30mNms", label: "RW 30", label_en: null })).toBe("RW 30");
    expect(itemLabel({ id: "local/rw_30mNms", label: null, label_en: null })).toBe(
      "local/rw_30mNms",
    );
  });
});
