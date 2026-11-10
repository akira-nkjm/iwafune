import {
  Boxes,
  Cpu,
  FileWarning,
  Gauge,
  Layers3,
  LayoutDashboard,
  Library,
  type LucideIcon,
  Network,
  Orbit,
  Radio,
  Ruler,
  Settings,
  Sun,
  Table2,
  Thermometer,
  Workflow,
} from "lucide-react";
import type { System } from "@/api/types";
import type { Dict } from "@/i18n";

/**
 * URL の検索部分の名前。ページが読み（`useDetailRoute`・系のページ）、下の `paths` が書く。
 * - `view`: 一覧と詳細のページの表示方法
 * - `adding`: 追加ダイアログを開いているか（値は `"1"`）
 * - `case`: 系のページの解析ケース
 */
export const SEARCH = { view: "view", adding: "new", case: "case" } as const;

/** 一覧のページの URL の下に、1 つのものの ID を足す（ID は encode する。受け取る側は React Router が decode した値を使う） */
export function itemPath(base: string, id: string): string {
  return `${base}/${encodeURIComponent(id)}`;
}

/** 検索部分を付ける（値が null・空のものは付けない） */
function withSearch(path: string, search: Record<string, string | null | undefined>): string {
  const params = new URLSearchParams();
  for (const [k, v] of Object.entries(search)) if (v) params.set(k, v);
  const text = params.toString();
  return text ? `${path}?${text}` : path;
}

/** 定義のページのタブ（単位は既定のタブで、URL に区間を足さない） */
export type DefsTab = "attributes" | "kinds";

/**
 * ページの URL。**ページの URL を組み立てるのはここだけ**（ルートの形は `routes.ts`。一致は nav.test.ts が確かめる）。
 * ID と解析ケースは encode する。
 */
export const paths = {
  overview: () => "/",
  components: () => "/design/components",
  component: (id: string) => itemPath(paths.components(), id),
  /** 部品の追加ダイアログを開いた部品のページ */
  addComponent: () => withSearch(paths.components(), { [SEARCH.adding]: "1" }),
  catalog: () => "/design/catalog",
  catalogItem: (id: string) => itemPath(paths.catalog(), id),
  diagram: () => "/design/diagram",
  modes: () => "/design/modes",
  defs: (tab?: DefsTab) => (tab ? `/design/defs/${tab}` : "/design/defs"),
  /** 系のページ。`tab` が無ければ結果のタブ、`kase` があればその解析ケースで開く */
  system: (system: System, at: { tab?: string | null; kase?: string | null } = {}) =>
    withSearch(at.tab ? `/systems/${system}/${at.tab}` : `/systems/${system}`, {
      [SEARCH.case]: at.kase,
    }),
  orbit: () => "/mission/orbit",
  cases: () => "/mission/cases",
  diagnostics: () => "/project/diagnostics",
  settings: () => "/project/settings",
};

export type NavItemKey = keyof Dict["nav"]["items"];
export type NavSectionKey = keyof Dict["nav"]["sections"];

export interface NavItem {
  to: string;
  /** 表示名は辞書（t.nav.items）から引く */
  key: NavItemKey;
  icon: LucideIcon;
  /** 系のページの項目なら、その系（先読みで系の解析を読む） */
  system?: System;
}

/** 系の項目（URL は `paths.system`） */
function systemItem(system: System & NavItemKey, icon: LucideIcon): NavItem {
  return { to: paths.system(system), key: system, icon, system };
}

export interface NavSection {
  /** 区分の開閉の保存と、見出しの辞書のキー（top は見出しを出さない） */
  id: NavSectionKey;
  heading: boolean;
  items: NavItem[];
}

/** サイドバーの並び（URL は `paths`） */
export const nav: NavSection[] = [
  {
    id: "top",
    heading: false,
    items: [{ to: paths.overview(), key: "overview", icon: LayoutDashboard }],
  },
  {
    id: "design",
    heading: true,
    items: [
      { to: paths.components(), key: "components", icon: Boxes },
      { to: paths.catalog(), key: "catalog", icon: Library },
      { to: paths.diagram(), key: "diagram", icon: Network },
      { to: paths.modes(), key: "modes", icon: Table2 },
      { to: paths.defs(), key: "defs", icon: Ruler },
    ],
  },
  {
    id: "systems",
    heading: true,
    items: [
      systemItem("power", Sun),
      systemItem("attitude", Gauge),
      systemItem("cdh", Cpu),
      systemItem("comm", Radio),
      systemItem("thermal", Thermometer),
      systemItem("structure", Layers3),
    ],
  },
  {
    id: "mission",
    heading: true,
    items: [
      { to: paths.orbit(), key: "orbit", icon: Orbit },
      { to: paths.cases(), key: "cases", icon: Workflow },
    ],
  },
  {
    id: "project",
    heading: true,
    items: [
      { to: paths.diagnostics(), key: "diagnostics", icon: FileWarning },
      { to: paths.settings(), key: "settings", icon: Settings },
    ],
  },
];

/**
 * ナビの項目が系のページか（項目の `system` で見る。系の項目のキーは系の綴りそのもの）。先読みの計画が系の解析を読むのに使う
 */
export function isSystemItem(key: NavItemKey): key is NavItemKey & System {
  return nav.some((s) => s.items.some((i) => i.key === key && i.system === key));
}
