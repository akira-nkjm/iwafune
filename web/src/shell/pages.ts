import { lazyPage as page } from "./lazy-page";

/**
 * ページの本体。routes/ のモジュールは `LazyRoute` でこれを描き、読む間はページの骨組みのスケルトンを出す。
 * ナビにホバーしたときと、最初のページが落ち着いたあとにも同じ load で先に読む（preload.ts）。
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
    () => import("@/features/mission/OrbitPage"),
    (m) => m.OrbitPage,
  ),
  cases: page(
    () => import("@/features/mission/CasesPage"),
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
