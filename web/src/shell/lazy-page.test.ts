import type { ComponentType } from "react";
import { describe, expect, it } from "vitest";
import { lazyPage } from "./lazy-page";

const Page: ComponentType = () => null;

describe("lazyPage", () => {
  it("読むのは 1 回だけで、読み終わったら current に本体が入る", async () => {
    let calls = 0;
    const page = lazyPage(
      async () => {
        calls += 1;
        return { Page };
      },
      (m) => m.Page,
    );
    expect(page.current).toBeNull();
    const [a, b] = await Promise.all([page.load(), page.load()]);
    expect(a).toBe(Page);
    expect(b).toBe(Page);
    expect(page.current).toBe(Page);
    await page.load();
    expect(calls).toBe(1);
  });

  it("読めなかったら次の呼び出しで読み直す", async () => {
    let calls = 0;
    const page = lazyPage(
      async () => {
        calls += 1;
        if (calls === 1) throw new Error("network");
        return { Page };
      },
      (m) => m.Page,
    );
    await expect(page.load()).rejects.toThrow("network");
    expect(page.current).toBeNull();
    await expect(page.load()).resolves.toBe(Page);
    expect(calls).toBe(2);
  });
});
