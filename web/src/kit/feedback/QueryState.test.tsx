import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it, vi } from "vitest";
import { findElement, fire } from "@/test/elements";
import { type QueryLike, QueryState, queryPhase } from "./QueryState";

const html = (query: QueryLike<number>, failure?: (m: string) => string) =>
  renderToStaticMarkup(
    <QueryState query={query} skeleton={<i>骨組み</i>} failure={failure}>
      {(n) => <b>値 {n}</b>}
    </QueryState>,
  );

describe("queryPhase", () => {
  it("値があれば、裏の読み直しに失敗していても値を出す", () => {
    expect(queryPhase({ data: 3, error: { message: "x" } })).toEqual({ phase: "ready", data: 3 });
    expect(queryPhase({ data: 0, error: null })).toEqual({ phase: "ready", data: 0 });
    expect(queryPhase({ data: undefined, error: { message: "x" } })).toEqual({
      phase: "failed",
      message: "x",
    });
    expect(queryPhase({ data: undefined, error: null })).toEqual({ phase: "loading" });
  });
});

describe("QueryState", () => {
  const refetch = vi.fn();

  it("読み込み中はスケルトンを読み込み中の印で包んで出す", () => {
    const out = html({ data: undefined, error: null, refetch });
    expect(out).toContain('aria-busy="true"');
    expect(out).toContain("骨組み");
    expect(out).not.toContain("値");
  });

  it("読めたら値を children に渡す", () => {
    expect(html({ data: 7, error: null, refetch })).toContain("値 7");
  });

  it("失敗は理由（既定は設計を読めない旨、または failure の文言）と再試行のボタン", () => {
    const out = html({ data: undefined, error: { message: "500" }, refetch });
    expect(out).toContain('role="alert"');
    expect(out).toContain("500");
    expect(
      html({ data: undefined, error: { message: "500" }, refetch }, (m) => `だめ ${m}`),
    ).toContain("だめ 500");
  });

  it("再試行のボタンは問い合わせを読み直す", () => {
    const again = vi.fn();
    const tree = QueryState({
      query: { data: undefined, error: { message: "500" }, refetch: again },
      skeleton: null,
      children: () => null,
    });
    // QueryFailure の中のボタンを探して押す（DOM を使わない）
    const failure = findElement(tree, (p) => typeof p.onRetry === "function");
    if (!failure) throw new Error("再試行の部品がありません");
    fire(failure, "onRetry");
    expect(again).toHaveBeenCalledOnce();
  });
});
