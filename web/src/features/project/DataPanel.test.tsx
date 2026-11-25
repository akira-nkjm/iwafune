import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it, vi } from "vitest";
import type { DataItem, DataStatus } from "@/api/types";
import { DataPanel } from "./DataPanel";

const state = vi.hoisted(() => ({ pending: undefined as string | undefined }));

const item = (id: string): DataItem => ({
  id,
  path: `/cache/${id}`,
  present: false,
  size: null,
  sha256_expected: null,
  verified: null,
  installed: false,
  install_error: null,
  source: "https://example.com/data",
  fetched_at: null,
  used_by: "Used by tests",
  license: null,
});
const status: DataStatus = {
  cache_dir: "/cache",
  ephemeris: "analytic",
  items: [item("de440"), item("space_weather")],
};

vi.mock("@tanstack/react-query", async (original) => ({
  ...(await original<typeof import("@tanstack/react-query")>()),
  useQuery: () => ({ data: status, error: null }),
}));
// 取得の呼び出しそのものは見ない。実行中の表示がどの行に出るかだけを見る
vi.mock("@/api/data", () => ({
  useFetchData: () => ({
    isPending: state.pending !== undefined,
    variables: state.pending,
    error: null,
    mutate() {},
  }),
}));

/** ボタンの文面を行の順に取り出す */
function buttons(): string[] {
  const html = renderToStaticMarkup(<DataPanel />);
  return [...html.matchAll(/<button[^>]*>([^<]*)<\/button>/g)].map((m) => m[1] ?? "");
}

describe("外部データの取得ボタン", () => {
  it("取得していなければ、どの行も取得のボタン", () => {
    state.pending = undefined;
    expect(buttons()).toEqual(["取得", "取得"]);
  });

  it("実行中の表示は押した行だけに出る", () => {
    state.pending = "space_weather";
    const [first, second] = buttons();
    expect(first).toBe("取得");
    expect(second).toBe("取得中…");
  });
});
