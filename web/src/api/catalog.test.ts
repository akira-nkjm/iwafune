import { afterEach, describe, expect, it, vi } from "vitest";
import { updateCatalog } from "./catalog";

afterEach(() => vi.unstubAllGlobals());

describe("共有カタログの更新", () => {
  it("POST /catalog/update に名前を渡す", async () => {
    const result = { catalogs: [], lock_changed: false, items: [] };
    const fetcher = vi.fn(async () => new Response(JSON.stringify(result), { status: 200 }));
    vi.stubGlobal("fetch", fetcher);
    expect(await updateCatalog({ name: "lab" })).toEqual(result);
    const [url, init] = fetcher.mock.calls[0] as unknown as [string, RequestInit];
    expect(url).toBe("/api/v1/catalog/update");
    expect(init.method).toBe("POST");
    expect(JSON.parse(String(init.body))).toEqual({ name: "lab" });
  });
});
