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
import type { Dict } from "@/i18n";

export type NavItemKey = keyof Dict["nav"]["items"];
export type NavSectionKey = keyof Dict["nav"]["sections"];

export interface NavItem {
  to: string;
  /** 表示名は辞書（t.nav.items）から引く */
  key: NavItemKey;
  icon: LucideIcon;
}

export interface NavSection {
  /** 区分の開閉の保存と、見出しの辞書のキー（top は見出しを出さない） */
  id: NavSectionKey;
  heading: boolean;
  items: NavItem[];
}

/** サイドバーの並び。ページの URL もここが正 */
export const nav: NavSection[] = [
  { id: "top", heading: false, items: [{ to: "/", key: "overview", icon: LayoutDashboard }] },
  {
    id: "design",
    heading: true,
    items: [
      { to: "/design/components", key: "components", icon: Boxes },
      { to: "/design/catalog", key: "catalog", icon: Library },
      { to: "/design/diagram", key: "diagram", icon: Network },
      { to: "/design/modes", key: "modes", icon: Table2 },
      { to: "/design/defs", key: "defs", icon: Ruler },
    ],
  },
  {
    id: "systems",
    heading: true,
    items: [
      { to: "/systems/power", key: "power", icon: Sun },
      { to: "/systems/attitude", key: "attitude", icon: Gauge },
      { to: "/systems/cdh", key: "cdh", icon: Cpu },
      { to: "/systems/comm", key: "comm", icon: Radio },
      { to: "/systems/thermal", key: "thermal", icon: Thermometer },
      { to: "/systems/structure", key: "structure", icon: Layers3 },
    ],
  },
  {
    id: "mission",
    heading: true,
    items: [
      { to: "/mission/orbit", key: "orbit", icon: Orbit },
      { to: "/mission/cases", key: "cases", icon: Workflow },
    ],
  },
  {
    id: "project",
    heading: true,
    items: [
      { to: "/project/diagnostics", key: "diagnostics", icon: FileWarning },
      { to: "/project/settings", key: "settings", icon: Settings },
    ],
  },
];
