import type { RouteConfigEntry } from "@react-router/dev/routes";
import { describe, expect, it } from "vitest";
import routes from "@/routes";
import { nav, paths } from "./nav";

/** routes.ts のルートの形（`design/components/:id?`）→ 道筋に合うかの正規表現 */
function patterns(entries: readonly RouteConfigEntry[]): RegExp[] {
  return entries.flatMap((e) => {
    const own =
      e.path === undefined && e.index
        ? [/^\/$/]
        : e.path && e.path !== "*"
          ? [
              new RegExp(
                `^/${e.path.replace(/\/:[a-z]+\?/g, "(?:/[^/]+)?").replace(/:[a-z]+/g, "[^/]+")}$`,
              ),
            ]
          : [];
    return [...own, ...patterns(e.children ?? [])];
  });
}

const known = patterns(routes);
const matches = (url: string) => {
  const path = new URL(url, "http://x").pathname;
  return known.some((p) => p.test(path));
};

describe("paths", () => {
  it("作る URL はどれも routes.ts のルートに当たる", () => {
    const urls = [
      ...nav.flatMap((s) => s.items.map((i) => i.to)),
      paths.component("RW_X"),
      paths.addComponent(),
      paths.catalogItem("local/rw"),
      paths.defs("attributes"),
      paths.defs("kinds"),
      paths.system("power", { tab: "breakdown", kase: "hot" }),
    ];
    for (const url of urls) expect(matches(url), url).toBe(true);
    expect(matches("/design/nowhere")).toBe(false);
  });

  it("ID は 1 つの区間として encode し、解析ケースは検索部分に入れる", () => {
    expect(paths.component("RW_X")).toBe("/design/components/RW_X");
    expect(paths.catalogItem("local/rw x")).toBe("/design/catalog/local%2Frw%20x");
    expect(paths.addComponent()).toBe("/design/components?new=1");
    expect(paths.system("thermal")).toBe("/systems/thermal");
    expect(paths.system("thermal", { tab: "series", kase: "hot&cold" })).toBe(
      "/systems/thermal/series?case=hot%26cold",
    );
    expect(paths.system("power", { tab: null, kase: null })).toBe("/systems/power");
  });
});
