import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it, vi } from "vitest";
import type { DesignView } from "@/api/types";
import { fixture } from "@/test/fixture";
import { ListDetailPage } from "./ListDetailPage";
import type { QueryLike } from "./QueryState";
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

type DesignQuery = QueryLike<DesignView>;

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

const refetch = () => undefined;
const loaded: DesignQuery = { data: fixture, error: null, refetch };

describe("ListDetailPage", () => {
  it("見出しとタブはいつも出し、読み込めなければ理由を出す", () => {
    const { html } = page({ data: undefined, error: { message: "500" }, refetch }, route());
    expect(html).toContain("見出し");
    expect(html).toContain("タブ");
    expect(html).toContain("500");
    expect(html).not.toContain("件");
  });

  it("読み込むまではスケルトン（読み込み中の印）を出す", () => {
    const { html } = page({ data: undefined, error: null, refetch }, route());
    expect(html).toContain('aria-busy="true"');
    // 読み込み中の印の中に、フォーカスできる境目を入れない
    expect(html).not.toContain('role="separator"');
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
