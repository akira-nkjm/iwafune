import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import type { ReactNode } from "react";
import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it, vi } from "vitest";
import type { CatalogSourceView } from "@/api/types";
import { SharedCatalogsButton } from "./SharedCatalogsButton";
import { neededCount, SharedCatalogsDialog } from "./SharedCatalogsDialog";

// Radix のダイアログの中身はポータルで、サーバー描画では出ない。ここでは中身の表示だけを見るので素通しにする
vi.mock("@/ui/dialog", () => {
  const Pass = ({ children }: { children?: ReactNode }) => <div>{children}</div>;
  return {
    Dialog: Pass,
    DialogContent: Pass,
    DialogHeader: Pass,
    DialogTitle: Pass,
    DialogDescription: Pass,
  };
});

function render(catalogs: CatalogSourceView[]): string {
  return renderToStaticMarkup(
    <QueryClientProvider client={new QueryClient()}>
      <SharedCatalogsDialog catalogs={catalogs} open onOpenChange={() => {}} />
    </QueryClientProvider>,
  );
}

const commit = "0123456789abcdef0123456789abcdef01234567";

describe("共有カタログのダイアログ", () => {
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

const sharedOf = (state: "ready" | "not_locked"): CatalogSourceView => ({
  name: "lab",
  editable: false,
  shared: { name: "lab", git: "https://example.com/c.git", rev: null, commit: null, state },
});

describe("見出しの共有カタログのボタン", () => {
  const button = (total: number, needed: number) =>
    renderToStaticMarkup(<SharedCatalogsButton total={total} needed={needed} onClick={() => {}} />);

  it("取得が要る共有カタログの数を、読み上げ用の名前つきの警告で出す", () => {
    expect(neededCount([sharedOf("not_locked"), sharedOf("ready")])).toBe(1);
    const html = button(2, 1);
    expect(html).toContain('aria-label="共有カタログ: 1 件が取得待ち"');
    expect(html).toContain("text-warn-ink");
  });

  it("すべて読み込めていれば警告を出さず、数だけを出す", () => {
    const html = button(2, 0);
    expect(html).not.toContain("text-warn-ink");
    expect(html).not.toContain("aria-label");
    expect(html).toContain(">2<");
  });
});
