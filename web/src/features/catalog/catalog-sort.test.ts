import { describe, expect, it } from "vitest";
import type { CatalogItemView } from "@/api/types";
import { catalogComparators } from "./catalog-sort";

const defs = {
  kinds: [
    { id: "a_kind", label: "Zeta", label_en: null },
    { id: "b_kind", label: "Alpha", label_en: null },
  ],
} as never;
const item = (over: Partial<CatalogItemView>) =>
  ({
    id: "x",
    scope: "local",
    kind: "wheel",
    label: null,
    label_en: null,
    manufacturer: null,
    part_number: null,
    maturity: null,
    users: [],
    ...over,
  }) as CatalogItemView;
const cmp = catalogComparators(defs, (v) => `M:${v}`);

describe("catalogComparators", () => {
  it("ID は数字を数として自然順に並べる", () => {
    expect(cmp.id(item({ id: "a2" }), item({ id: "a10" }), "ascending")).toBeLessThan(0);
    expect(cmp.id(item({ id: "a2" }), item({ id: "a10" }), "descending")).toBeGreaterThan(0);
  });

  it("使用は個数で比べる", () => {
    const two = item({ users: ["a", "b"] });
    expect(cmp.used(two, item({ users: ["a"] }), "ascending")).toBeGreaterThan(0);
  });

  it("値の無い品は向きによらず末尾", () => {
    const set = item({ manufacturer: "Acme" });
    const none = item({});
    expect(cmp.maker(none, set, "ascending")).toBeGreaterThan(0);
    expect(cmp.maker(none, set, "descending")).toBeGreaterThan(0);
    expect(cmp.maturity(item({ maturity: "new" }), none, "descending")).toBeLessThan(0);
  });

  it("種別は表示名で比べる", () => {
    expect(cmp.kind(item({ kind: "b_kind" }), item({ kind: "a_kind" }), "ascending")).toBeLessThan(
      0,
    );
  });
});
