import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";
import type { CatalogSourceView } from "@/api/types";
import { SharedCatalogs } from "./SharedCatalogs";

function render(catalogs: CatalogSourceView[]): string {
  return renderToStaticMarkup(
    <QueryClientProvider client={new QueryClient()}>
      <SharedCatalogs catalogs={catalogs} />
    </QueryClientProvider>,
  );
}

const commit = "0123456789abcdef0123456789abcdef01234567";

describe("共有カタログの lock の状態", () => {
  it("共有カタログが無ければ、足すボタンと説明だけを出す（取得のボタンは出さない）", () => {
    const html = render([{ name: "local", editable: true, shared: null }]);
    expect(html).toContain("共有カタログを足す");
    expect(html).toContain("共有カタログはありません");
    expect(html).not.toContain("取得</button>");
  });

  it("commit と状態（API の判定）を出し、取得が要るものには次にすることを添える", () => {
    const html = render([
      {
        name: "lab",
        editable: false,
        shared: {
          name: "lab",
          git: "https://example.com/catalog.git",
          rev: "v1",
          commit,
          state: "ready",
        },
      },
      {
        name: "team",
        editable: false,
        shared: {
          name: "team",
          git: "https://example.com/team.git",
          rev: null,
          commit: null,
          state: "not_locked",
        },
      },
    ]);
    expect(html).toContain(commit);
    expect(html).toContain("https://example.com/catalog.git");
    expect(html).toContain("rev v1");
    expect(html).toContain("catalog.lock に記録がありません");
    expect(html).toContain("<button");
    // 共有カタログごとに外すボタン（確認のダイアログを開く）
    expect(html).toContain('aria-label="lab を外す"');
    expect(html).toContain('aria-label="team を外す"');
  });
});
