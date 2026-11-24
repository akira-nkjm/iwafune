import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it, vi } from "vitest";
import type { Op } from "@/api/types";
import { getT } from "@/i18n";
import { OpPreview } from "./OpPreview";

// 置き換える操作が既存のレコードから落とすキーは、API の結果の `removed` をそのまま出す（web で差分を取らない）
const OPS: Op[] = [{ op: "set_case", case: { id: "hot", mode: "NOMINAL" } as never }];

vi.mock("./use-op-check", () => ({
  useOpCheck: () => ({
    pending: false,
    previewError: null,
    result: {
      revision: "r",
      previous_revision: "p",
      dry_run: true,
      changed: [],
      diagnostics: [],
      removed: [{ path: "cases/hot.toml", keys: ["beta", "orbits"] }],
      cli: ["fune case set hot --mode NOMINAL"],
    },
  }),
}));
vi.mock("@/i18n", async (original) => ({
  ...(await original<typeof import("@/i18n")>()),
  useT: () => getT(),
}));

describe("OpPreview", () => {
  it("lists the keys the operation removes from an existing record", () => {
    const html = renderToStaticMarkup(<OpPreview ops={OPS} />);
    expect(html).toContain(getT().cli.removedKeys("cases/hot.toml", "beta, orbits"));
    expect(html).toContain("cases/hot.toml");
    expect(html).toContain("beta, orbits");
  });
});
