import { preloadSystem } from "@/features/systems/preload";
import { isSystemItem, type NavItemKey } from "@/lib/nav";
import { lazyPage as page, quietly } from "@/shell/lazy-page";
import type { PreloadPlan } from "@/shell/preload";

/**
 * ページの登録: ページの本体（ページごとに別のチャンク）と先読みの計画。routes/ のモジュールは `LazyRoute` で本体を描き、
 * 読む間はページの骨組みのスケルトンを出す。ナビにホバーしたときと、最初のページが落ち着いたあとにも同じ load で
 * 先に読む（`preloadPlan` を `routes/layout.tsx` が shell の `Frame` に渡す。shell は features を import しない）。
 */
export const pages = {
  overview: page(
    () => import("@/features/overview/OverviewPage"),
    (m) => m.OverviewPage,
  ),
  components: page(
    () => import("@/features/components/ComponentsPage"),
    (m) => m.ComponentsPage,
  ),
  catalog: page(
    () => import("@/features/catalog/CatalogPage"),
    (m) => m.CatalogPage,
  ),
  diagram: page(
    () => import("@/features/diagram/DiagramPage"),
    (m) => m.DiagramPage,
  ),
  modes: page(
    () => import("@/features/modes/ModesPage"),
    (m) => m.ModesPage,
  ),
  defs: page(
    () => import("@/features/defs/DefsPage"),
    (m) => m.DefsPage,
  ),
  system: page(
    () => import("@/features/systems/SystemPage"),
    (m) => m.SystemPage,
  ),
  orbit: page(
    () => import("@/features/conditions/OrbitPage"),
    (m) => m.OrbitPage,
  ),
  cases: page(
    () => import("@/features/conditions/CasesPage"),
    (m) => m.CasesPage,
  ),
  diagnostics: page(
    () => import("@/features/project/DiagnosticsPage"),
    (m) => m.DiagnosticsPage,
  ),
  settings: page(
    () => import("@/features/project/SettingsPage"),
    (m) => m.SettingsPage,
  ),
};

/**
 * ナビの項目にホバー・フォーカスしたときに、そのページの本体と、時間のかかるもの（系の解析・追加の表示のチャンク）を
 * 先に読む。系のページは 1 つの本体を系ごとの項目で共有する。
 */
function preloadItem(key: NavItemKey): void {
  if (isSystemItem(key)) {
    quietly(pages.system.load());
    for (const p of preloadSystem(key)) quietly(p);
    return;
  }
  quietly(pages[key].load());
}

/** 先読みの計画。大きなチャンク（系統図の React Flow）は空き時間には読まない（ホバーで読む） */
export const preloadPlan: PreloadPlan = {
  item: preloadItem,
  idle: [
    pages.overview,
    pages.components,
    pages.catalog,
    pages.modes,
    pages.defs,
    pages.system,
    pages.orbit,
    pages.cases,
    pages.diagnostics,
    pages.settings,
  ],
};
