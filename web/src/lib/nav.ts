import {
  BookOpen,
  Box,
  Boxes,
  ClipboardCheck,
  Cpu,
  FileWarning,
  Gauge,
  Layers3,
  LayoutDashboard,
  Library,
  type LucideIcon,
  Milestone,
  Network,
  Orbit,
  Radio,
  Rocket,
  Ruler,
  Settings,
  Sun,
  Table2,
  Telescope,
  Thermometer,
  Workflow,
} from "lucide-react";
import type { AnalysisArea, DefsView, System } from "@/api/types";
import type { Dict } from "@/i18n";
import { systemTitle } from "./design";

/**
 * URL の検索部分の名前。ページが読み（`useDetailRoute`・系のページ）、下の `paths` が書く。
 * - `view`: 一覧と詳細のページの表示方法
 * - `adding`: 追加ダイアログを開いているか（値は `"1"`）
 * - `group`: カタログのツリーの分け方
 * - `case`: 系のページの解析ケース
 * - `maneuver`: 推進のマヌーバのタブで強調するマヌーバの ID
 * - `phase`: フェーズのページで強調するフェーズの ID
 * - `diagramHiddenSystem`: 系統図で隠す系（繰り返し可、空は全表示）
 */
export const SEARCH = {
  view: "view",
  adding: "new",
  group: "group",
  case: "case",
  maneuver: "maneuver",
  phase: "phase",
  diagramHiddenSystem: "hidden-system",
} as const;

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

/** 要求のページのタブ（適合表は既定のタブで、URL に足さない） */
export type RequirementsTab = "list";

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
  layout: () => "/design/layout",
  requirements: (tab?: RequirementsTab) =>
    tab ? `/design/requirements/${tab}` : "/design/requirements",
  defs: (tab?: DefsTab) => (tab ? `/project/defs/${tab}` : "/project/defs"),
  /** 系のページ。`tab` が無ければ結果のタブ、`kase` があればその解析ケースで開く */
  system: (
    system: System,
    at: { tab?: string | null; kase?: string | null; maneuver?: string | null } = {},
  ) =>
    withSearch(at.tab ? `/systems/${system}/${at.tab}` : `/systems/${system}`, {
      [SEARCH.case]: at.kase,
      [SEARCH.maneuver]: at.maneuver,
    }),
  /** 推進のページのマヌーバのタブ（一覧） */
  maneuvers: () => paths.system("propulsion", { tab: "maneuvers" }),
  /** マヌーバのタブで、その ID の行を強調して開く */
  maneuver: (id: string) => paths.system("propulsion", { tab: "maneuvers", maneuver: id }),
  /** 解析の区分に対応するページ。 */
  area: (area: AnalysisArea): string =>
    area.kind === "system" ? paths.system(area.value) : paths.orbit(),
  orbit: () => "/conditions/orbit",
  cases: () => "/conditions/cases",
  phases: () => "/conditions/phases",
  /** フェーズのページで、その ID の行を強調して開く */
  phase: (id: string) => withSearch(paths.phases(), { [SEARCH.phase]: id }),
  diagnostics: () => "/project/diagnostics",
  settings: () => "/project/settings",
  sources: () => "/project/sources",
};

export type NavItemKey = keyof Dict["nav"]["items"] | System;
export type NavSectionKey = keyof Dict["nav"]["sections"];

export interface NavItem {
  to: string;
  /** 表示名は navItemTitle で語彙または辞書から引く */
  key: NavItemKey;
  icon: LucideIcon;
  /** 系のページの項目なら、その系（先読みで系の解析を読む） */
  system?: System;
}

/** 系のアイコンの対応。項目の一覧と順序は API が返す。 */
const systemIcons: Record<System, LucideIcon> = {
  aocs: Gauge,
  cdh: Cpu,
  comm: Radio,
  payload: Telescope,
  power: Sun,
  propulsion: Rocket,
  structure: Layers3,
  thermal: Thermometer,
};

/** ナビの項目の表示名。系の語彙は API の labels.section から引く。 */
export function navItemTitle(item: NavItem, dict: Dict, defs?: DefsView): string {
  return isSystemItem(item.key) ? systemTitle(item.key, defs) : dict.nav.items[item.key];
}

export interface NavSection {
  /** 区分の開閉の保存と、見出しの辞書のキー（top は見出しを出さない） */
  id: NavSectionKey;
  heading: boolean;
  items: NavItem[];
}

/** サイドバーの並び（URL は `paths`） */
export const nav = (systems: readonly System[]): NavSection[] => [
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
      { to: paths.layout(), key: "layout", icon: Box },
      { to: paths.requirements(), key: "requirements", icon: ClipboardCheck },
    ],
  },
  {
    id: "systems",
    heading: true,
    items: systems.map((system) => ({
      to: paths.system(system),
      key: system,
      icon: systemIcons[system],
      system,
    })),
  },
  {
    id: "conditions",
    heading: true,
    items: [
      { to: paths.orbit(), key: "orbit", icon: Orbit },
      { to: paths.cases(), key: "cases", icon: Workflow },
      { to: paths.phases(), key: "phases", icon: Milestone },
    ],
  },
  {
    id: "project",
    heading: true,
    items: [
      { to: paths.diagnostics(), key: "diagnostics", icon: FileWarning },
      { to: paths.settings(), key: "settings", icon: Settings },
      { to: paths.defs(), key: "defs", icon: Ruler },
      { to: paths.sources(), key: "sources", icon: BookOpen },
    ],
  },
];

/**
 * ナビのキーが系のページか。アイコンの対応で型を確かめ、先読みの計画が系の解析を読むのに使う
 */
export function isSystemItem(key: NavItemKey): key is NavItemKey & System {
  return Object.hasOwn(systemIcons, key);
}

/** 系統図で隠す系。指定なしは構造を隠す。空の指定は全表示。未知の系は無視する。 */
export function diagramHiddenSystems(
  params: URLSearchParams,
  systems: readonly System[],
): ReadonlySet<System> {
  const values = params.has(SEARCH.diagramHiddenSystem)
    ? params.getAll(SEARCH.diagramHiddenSystem)
    : ["structure"];
  return new Set(systems.filter((system) => values.includes(system)));
}

/** 系統図の表示状態だけを書き換える。空の指定も残し、再読み込みで既定に戻らないようにする。 */
export function withDiagramHiddenSystems(
  params: URLSearchParams,
  hidden: ReadonlySet<System>,
): URLSearchParams {
  const next = new URLSearchParams(params);
  next.delete(SEARCH.diagramHiddenSystem);
  for (const system of hidden) next.append(SEARCH.diagramHiddenSystem, system);
  if (hidden.size === 0) next.set(SEARCH.diagramHiddenSystem, "");
  return next;
}
