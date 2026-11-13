import { renderToStaticMarkup } from "react-dom/server";
import { afterEach, expect, it, vi } from "vitest";
import { getT, setLang } from "@/i18n";
import { fixture } from "@/test/fixture";
import { PlacementSection } from "./PlacementSection";

vi.mock("@/api/ops", () => ({
  useApplyOps: () => ({ mutate: vi.fn(), isPending: false }),
  useCommandOps: () => ({ ops: null, error: null }),
}));
vi.mock("@/i18n", async (original) => ({
  ...(await original<typeof import("@/i18n")>()),
  useT: () => getT(),
}));
afterEach(() => setLang("ja"));

it.each(["ja", "en"] as const)(
  "%s: 配置のラベルは欄の上で折り返し、入力に固定幅を付けない",
  (lang) => {
    setLang(lang);
    const t = getT().components.placement;
    const html = renderToStaticMarkup(
      <PlacementSection design={fixture} part={fixture.parts[0]!} />,
    );
    for (const label of [t.mountOn, t.position, t.orientation, t.bodyFrame]) {
      expect(html).toContain(`>${label}</label>`);
    }
    expect(html).not.toContain("truncate");
    expect(html).not.toMatch(/w-(28|52)/);
    expect(html).toContain("wrap-anywhere");
    expect(html).toContain("grid min-w-0 content-start gap-1");
    expect(html).toContain(`aria-label="${t.position}"`);
    expect(html).toContain(`aria-label="${t.orientation}"`);
  },
);
