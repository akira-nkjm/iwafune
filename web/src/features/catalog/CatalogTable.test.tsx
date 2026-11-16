import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";
import type { CatalogItemView } from "@/api/types";
import { fixture } from "@/test/fixture";
import { CatalogFields } from "./CatalogFields";
import { CatalogTable } from "./CatalogTable";

const item: CatalogItemView = {
  id: "local/rw_10mNms",
  scope: "local",
  name: "rw_10mNms",
  dir: [],
  editable: true,
  kind: "reaction_wheel",
  label: null,
  label_en: null,
  manufacturer: null,
  part_number: null,
  maturity: null,
  tags: ["flight_proven", "国産"],
  tags_text: "flight_proven,国産",
  datasheet: null,
  datasheet_link: null,
  source: null,
  ports: {},
  states: [],
  values: {},
  paths: {},
  users: [],
};

describe("カタログ品のタグ", () => {
  it("一覧にタグの列を出し、タグを小さなピルで並べる", () => {
    const html = renderToStaticMarkup(
      <QueryClientProvider client={new QueryClient()}>
        <CatalogTable design={fixture} items={[item]} selected={null} onSelect={() => {}} />
      </QueryClientProvider>,
    );
    expect(html).toContain("flight_proven");
    expect(html).toContain("国産");
  });

  it("基本の欄にタグの行を出し、API の文字列をそのまま欄に入れる", () => {
    const html = renderToStaticMarkup(
      <QueryClientProvider client={new QueryClient()}>
        <CatalogFields item={item} />
      </QueryClientProvider>,
    );
    expect(html).toContain('value="flight_proven,国産"');
  });
});
