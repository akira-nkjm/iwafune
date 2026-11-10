import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it, vi } from "vitest";
import { fixture } from "@/test/fixture";
import { type DesignQuery, ListDetailPage } from "./ListDetailPage";
import type { DetailRoute } from "./use-detail-route";

function route(over: Partial<DetailRoute> = {}): DetailRoute {
  return {
    view: "tree",
    tree: true,
    selected: null,
    select: vi.fn(),
    adding: false,
    setAdding: vi.fn(),
    ...over,
  };
}

function page(design: DesignQuery, r: DetailRoute, detail = vi.fn(() => <p>詳細</p>)) {
  const html = renderToStaticMarkup(
    <ListDetailPage
      header={<h1>見出し</h1>}
      tabs={<nav>タブ</nav>}
      design={design}
      route={r}
      hint="選んでください"
      list={(d) => <ul>{d.parts.length} 件</ul>}
      detail={detail}
    />,
  );
  return { html, detail };
}

const loaded: DesignQuery = { data: fixture, isError: false, error: null };

describe("ListDetailPage", () => {
  it("見出しとタブはいつも出し、読み込めなければ理由を出す", () => {
    const { html } = page({ data: undefined, isError: true, error: { message: "500" } }, route());
    expect(html).toContain("見出し");
    expect(html).toContain("タブ");
    expect(html).toContain("500");
    expect(html).not.toContain("件");
  });

  it("読み込むまではスケルトン（読み込み中の印）を出す", () => {
    const { html } = page({ data: undefined, isError: false, error: null }, route());
    expect(html).toContain('aria-busy="true"');
  });

  it("読み込んだら一覧を出し、何も選んでいなければ詳細の欄に案内を出す", () => {
    const { html, detail } = page(loaded, route());
    expect(html).toContain(`${fixture.parts.length} 件`);
    expect(html).toContain("選んでください");
    expect(detail).not.toHaveBeenCalled();
  });

  it("URL で選んだ ID の詳細を作る", () => {
    const { html, detail } = page(loaded, route({ selected: "RW_X" }));
    expect(detail).toHaveBeenCalledWith(fixture, "RW_X");
    expect(html).toContain("詳細");
    expect(html).not.toContain("選んでください");
  });
});
