import { describe, expect, it } from "vitest";
import { buildSidebar } from "./sidebar.ts";

describe("buildSidebar", () => {
  it("あるページだけを固定の順に並べ、空のグループは出さない", () => {
    const have = new Set(["/d/guide/install.md", "/d/model.md", "/d/guide/index.md"]);
    const s = buildSidebar("/d", (f) => have.has(f));
    expect(s.map((g) => g.label)).toEqual(["Start here", "Reference"]);
    expect(s[0]?.items).toEqual([{ slug: "guide" }, { slug: "guide/install" }]);
    expect(s[1]?.items).toEqual([{ slug: "model" }]);
  });
  it("何も無くても壊れない", () => {
    expect(buildSidebar("/d", () => false)).toEqual([]);
  });
});
