import type { RouteConfigEntry } from "@react-router/dev/routes";
import { describe, expect, it } from "vitest";
import type { System } from "@/api/types";
import { en } from "@/i18n/en";
import { ja } from "@/i18n/ja";
import routes from "@/routes";
import { fixture } from "@/test/fixture";
import {
  diagramHiddenSystems,
  isSystemItem,
  nav,
  navItemTitle,
  paths,
  SEARCH,
  withDiagramHiddenSystems,
} from "./nav";

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
  it("配置は設計のモードの次にあり、日英の表示名と専用ルートを持つ", () => {
    const items = nav(fixture.defs.systems).find((s) => s.id === "design")?.items ?? [];
    const modes = items.findIndex((i) => i.key === "modes");
    const layout = items[modes + 1];
    expect(layout?.key).toBe("layout");
    expect(layout?.to).toBe(paths.layout());
    expect(paths.layout()).toBe("/design/layout");
    expect(matches(paths.layout())).toBe(true);
    expect(routes[0]?.children?.find((r) => r.path === "design/layout")?.file).toBe(
      "routes/layout3d.tsx",
    );
    expect(isSystemItem("layout")).toBe(false);
    if (!layout) throw new Error("missing layout nav item");
    expect(navItemTitle(layout, ja)).toBe("配置");
    expect(navItemTitle(layout, en)).toBe("Layout");
  });

  it("単位と定義はプロジェクトの設定の下にある", () => {
    const sections = nav(fixture.defs.systems);
    const design = sections.find((s) => s.id === "design")?.items ?? [];
    const project = sections.find((s) => s.id === "project")?.items ?? [];
    expect(design.some((i) => i.key === "defs")).toBe(false);
    expect(project.map((i) => i.key)).toEqual(["diagnostics", "settings", "defs", "sources"]);
    expect(paths.defs()).toBe("/project/defs");
    expect(matches(paths.defs("attributes"))).toBe(true);
  });

  it("要求は設計の下のページで、適合表が既定のタブ", () => {
    expect(paths.requirements()).toBe("/design/requirements");
    expect(paths.requirements("list")).toBe("/design/requirements/list");
    expect(matches(paths.requirements())).toBe(true);
    expect(matches(paths.requirements("list"))).toBe(true);
    const design = nav(fixture.defs.systems).find((s) => s.id === "design")?.items ?? [];
    const item = design.find((i) => i.key === "requirements");
    expect(item?.to).toBe(paths.requirements());
    expect(item && navItemTitle(item, ja)).toBe("要求");
    expect(item && navItemTitle(item, en)).toBe("Requirements");
  });

  it("出典の台帳はプロジェクトの下のページ", () => {
    expect(paths.sources()).toBe("/project/sources");
    expect(matches(paths.sources())).toBe(true);
    const project = nav(fixture.defs.systems).find((s) => s.id === "project")?.items ?? [];
    const item = project.find((i) => i.key === "sources");
    expect(item?.to).toBe(paths.sources());
    expect(item && navItemTitle(item, en)).toBe("Sources");
  });

  it("前提条件の URL と解析区分の開き先", () => {
    expect(paths.orbit()).toBe("/conditions/orbit");
    expect(paths.cases()).toBe("/conditions/cases");
    expect(paths.area({ kind: "condition", value: "orbit" })).toBe(paths.orbit());
    expect(paths.area({ kind: "system", value: "power" })).toBe(paths.system("power"));
  });
  it("作る URL はどれも routes.ts のルートに当たる", () => {
    const urls = [
      ...nav(fixture.defs.systems).flatMap((s) => s.items.map((i) => i.to)),
      paths.component("RW_X"),
      paths.addComponent(),
      paths.catalogItem("local/rw"),
      paths.defs("attributes"),
      paths.defs("kinds"),
      paths.system("power", { tab: "series", kase: "hot" }),
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

describe("nav", () => {
  it("ナビの項目はすべて違うアイコンを使う", () => {
    const icons = nav(fixture.defs.systems).flatMap((s) => s.items.map((i) => i.icon));
    expect(new Set(icons).size).toBe(icons.length);
  });
});

describe("isSystemItem", () => {
  it("API が返した系の一覧と順序に従う", () => {
    const systems = [...fixture.defs.systems].reverse();
    expect(
      nav(systems)
        .find((s) => s.id === "systems")
        ?.items.map((i) => i.system),
    ).toEqual(systems);
    expect(nav([]).find((s) => s.id === "systems")?.items).toEqual([]);
  });
  it("系の項目だけが系のページ（キーは系の綴り）", () => {
    expect(isSystemItem("thermal")).toBe(true);
    expect(isSystemItem("overview")).toBe(false);
    expect(isSystemItem("modes")).toBe(false);
    const systems = nav(fixture.defs.systems).flatMap((s) =>
      s.items.flatMap((i) => (i.system ? [i.system] : [])),
    );
    expect(systems).toEqual(fixture.defs.systems);
    expect(systems.every((s) => isSystemItem(s))).toBe(true);
  });
});

describe("系統図の URL の表示状態", () => {
  it("指定なしは構造だけを隠す。未知の系を無視し API にない系を作らない", () => {
    expect([...diagramHiddenSystems(new URLSearchParams(), fixture.defs.systems)]).toEqual([
      "structure",
    ]);
    expect([...diagramHiddenSystems(new URLSearchParams(), ["thermal"])]).toEqual([]);
    expect([
      ...diagramHiddenSystems(
        new URLSearchParams("hidden-system=unknown&hidden-system=thermal&hidden-system=thermal"),
        fixture.defs.systems,
      ),
    ]).toEqual(["thermal"]);
  });
  it("複数の系と全非表示を再読み込みでき、他の検索部分を保つ", () => {
    for (const hidden of [
      new Set<System>(["thermal", "aocs"]),
      new Set(fixture.defs.systems),
      new Set<System>(),
    ]) {
      const original = new URLSearchParams("view=tree&other=keep");
      const next = withDiagramHiddenSystems(original, hidden);
      expect(
        diagramHiddenSystems(new URLSearchParams(next.toString()), fixture.defs.systems),
      ).toEqual(hidden);
      expect(next.get("view")).toBe("tree");
      expect(next.get("other")).toBe("keep");
      expect(original.has(SEARCH.diagramHiddenSystem)).toBe(false);
    }
  });
});
