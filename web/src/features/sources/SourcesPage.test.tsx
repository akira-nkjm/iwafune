import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";
import type { SourceView } from "@/api/types";
import { SourcesTable } from "./SourcesPage";

const base: SourceView = {
  id: "ds_web",
  kind: "datasheet",
  link: "https://example.com/rw.pdf",
  file: "sources/ds_web.toml",
  ref: "https://example.com/rw.pdf",
  revision: "Rev. C",
  retrieved: "2026-10-15",
  sha256: "9f86d081884c7d659a2feaa0c55ad015a3bf4f1b2b0b822cd15d6c15b0f00a08",
  distribution: "restricted",
};

function render(sources: SourceView[]): string {
  return renderToStaticMarkup(
    <QueryClientProvider client={new QueryClient()}>
      <SourcesTable sources={sources} onEdit={() => {}} />
    </QueryClientProvider>,
  );
}

describe("出典の表", () => {
  it("Web の URL（API の link）だけを新しいタブで開くリンクにする", () => {
    const html = render([
      base,
      { ...base, id: "ds_path", link: null, ref: "docs/datasheets/rw.pdf", file: "x" },
    ]);
    expect(html).toContain('href="https://example.com/rw.pdf"');
    expect(html).toContain('target="_blank"');
    expect(html).toContain('rel="noopener noreferrer"');
    expect(html.match(/<a /g)?.length).toBe(1);
    expect(html).toContain("docs/datasheets/rw.pdf");
    expect(html).toContain("Rev. C");
    expect(html).toContain("2026-10-15");
  });
});
