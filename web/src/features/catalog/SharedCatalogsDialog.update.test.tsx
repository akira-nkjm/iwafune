import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import type { ReactNode } from "react";
import { renderToStaticMarkup } from "react-dom/server";
import { beforeEach, describe, expect, it, vi } from "vitest";
import type { CatalogSourceView, CatalogSyncResult } from "@/api/types";
import { SharedCatalogsDialog } from "./SharedCatalogsDialog";

const state = vi.hoisted(() => ({
  data: undefined as unknown,
  error: null as { message: string } | null,
  pending: false,
}));

// 更新の呼び出しそのものは api/catalog.test.ts で見る。ここは返り値・エラー・実行中の表示だけを見る
vi.mock("@/api/catalog", () => ({
  useFetchCatalogs: () => ({ isPending: false, error: null, mutate() {}, reset() {} }),
  useUpdateCatalog: () => ({
    isPending: state.pending,
    error: state.error,
    data: state.data,
    variables: state.data || state.pending ? { name: "lab" } : undefined,
    mutate() {},
    reset() {},
  }),
}));
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

const catalogs: CatalogSourceView[] = [
  { name: "local", editable: true, shared: null },
  {
    name: "lab",
    editable: false,
    shared: {
      name: "lab",
      git: "https://example.com/c.git",
      rev: "main",
      commit: "b2",
      state: "ready",
    },
  },
];

function render(): string {
  return renderToStaticMarkup(
    <QueryClientProvider client={new QueryClient()}>
      <SharedCatalogsDialog catalogs={catalogs} open onOpenChange={() => {}} />
    </QueryClientProvider>,
  );
}

describe("共有カタログの更新ボタンと結果", () => {
  beforeEach(() => {
    state.data = undefined;
    state.error = null;
    state.pending = false;
  });

  it("共有カタログごとに更新ボタンを出し（自前のカタログには出さない）、取得との違いを 1 行で説明する", () => {
    const html = render();
    expect(html).toContain('aria-label="lab を最新の commit に更新"');
    expect(html).not.toContain("local を最新");
    expect(html).toContain("更新は記録を最新の commit に進めます");
  });

  it("追加・削除・変わった値（API の整形済みの文字列）を出す", () => {
    const result: CatalogSyncResult = {
      catalogs: [{ name: "lab", git: "g", rev: "main", commit: "b2", previous: "a1" }],
      lock_changed: true,
      items: [
        { id: "lab/new", change: "added", values: [] },
        { id: "lab/old", change: "removed", values: [] },
        {
          id: "lab/rw",
          change: "changed",
          values: [{ key: "attr.mass", before: "150 g", after: "160 g" }],
        },
      ],
    };
    state.data = result;
    const html = render();
    expect(html).toContain("lab: a1 → b2");
    expect(html).toContain("lab/new");
    expect(html).toContain("lab/old");
    expect(html).toContain("attr.mass: 150 g → 160 g");
    expect(html).not.toContain("すでに最新です");
  });

  it("何も変わらなければ「すでに最新です」を出す", () => {
    state.data = {
      catalogs: [{ name: "lab", git: "g", rev: null, commit: "b2", previous: "b2" }],
      lock_changed: false,
      items: [],
    } satisfies CatalogSyncResult;
    expect(render()).toContain("すでに最新です");
  });

  it("失敗したら API のエラーを出し、実行中はボタンを止める", () => {
    state.error = { message: "remote unreachable" };
    expect(render()).toContain("remote unreachable");
    state.error = null;
    state.pending = true;
    const html = render();
    expect(html).toContain("更新中…");
    expect(html).toContain("disabled");
  });
});
