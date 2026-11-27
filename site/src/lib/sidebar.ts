/**
 * サイドバーの組み立て。並びは固定で、docs/ に実在するページだけを入れる
 * （まだ書かれていないページがあってもビルドが壊れない）。
 */
import fs from "node:fs";
import path from "node:path";

/** サイドバーの 1 グループ。`slug` は docs/ からの相対（拡張子なし）、`index` は `guide/index` → `guide`。 */
export interface GroupSpec {
  label: string;
  ja: string;
  slugs: string[];
}

export const GROUPS: GroupSpec[] = [
  {
    label: "Start here",
    ja: "はじめに",
    slugs: [
      "guide/index",
      "guide/install",
      "guide/quickstart",
      "guide/first-satellite",
      "guide/concepts",
    ],
  },
  { label: "Using iwafune", ja: "使い方", slugs: ["guide/cli", "guide/gui"] },
  {
    label: "Analyses",
    ja: "解析",
    slugs: [
      "analyses/index",
      // 分野は英語の名前のアルファベット順
      "analyses/aocs",
      "analyses/budgets",
      "analyses/comm",
      "analyses/orbit",
      "analyses/power",
      "analyses/propulsion",
      "analyses/radiation",
      "analyses/structure",
      "analyses/thermal",
    ],
  },
  {
    label: "Reference",
    ja: "リファレンス",
    slugs: ["model", "report", "trapped-radiation"],
  },
  {
    label: "Internals",
    ja: "内部のしくみ",
    slugs: [
      "architecture-overview",
      "architecture",
      "architecture/registry",
      "development",
      "roadmap",
    ],
  },
];

/** Starlight のサイドバーの項目（必要な形だけ）。 */
export interface SidebarGroup {
  label: string;
  translations: { ja: string };
  items: { slug: string }[];
}

/** docs/ にあるページだけで、サイドバーを組む。空のグループは出さない。 */
export function buildSidebar(
  docs: string,
  exists: (file: string) => boolean = fs.existsSync,
): SidebarGroup[] {
  return GROUPS.map((g) => ({
    label: g.label,
    translations: { ja: g.ja },
    items: g.slugs
      .filter((s) => exists(path.join(docs, `${s}.md`)))
      .map((s) => ({ slug: s.replace(/\/index$/, "") })),
  })).filter((g) => g.items.length > 0);
}
