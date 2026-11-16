import { renderToStaticMarkup } from "react-dom/server";
import { MemoryRouter } from "react-router";
import { describe, expect, it } from "vitest";
import { CATALOG_GROUPINGS } from "@/kit/tree/tree-nodes";
import { SEARCH } from "@/lib/nav";
import { useCatalogGrouping } from "./use-catalog-grouping";

function Probe() {
  return <span>{useCatalogGrouping()[0]}</span>;
}
const read = (search: string) =>
  renderToStaticMarkup(
    <MemoryRouter initialEntries={[`/design/catalog${search}`]}>
      <Probe />
    </MemoryRouter>,
  );

describe("useCatalogGrouping", () => {
  it("すべての分け方が URL の ?group= から読める（tag と system を含む）", () => {
    expect(CATALOG_GROUPINGS).toContain("tag");
    expect(CATALOG_GROUPINGS).toContain("system");
    for (const g of CATALOG_GROUPINGS) {
      const search = new URLSearchParams({ [SEARCH.group]: g }).toString();
      expect(read(`?${search}`)).toBe(`<span>${g}</span>`);
    }
  });
  it("不明な値と無いときは catalog", () => {
    expect(read("?group=nope")).toBe("<span>catalog</span>");
    expect(read("")).toBe("<span>catalog</span>");
  });
});
