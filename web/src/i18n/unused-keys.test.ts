import { readdirSync, readFileSync, statSync } from "node:fs";
import { join } from "node:path";
import { describe, expect, it } from "vitest";
import { dictionaries } from "./index";

type Shape = Record<string, unknown>;

/** 辞書の葉（文字列か差し込み関数）のパス "a.b.c" を全部並べる */
function leaves(o: Shape, prefix = ""): string[] {
  return Object.entries(o).flatMap(([k, v]) => {
    const path = prefix ? `${prefix}.${k}` : k;
    return v && typeof v === "object" ? leaves(v as Shape, path) : [path];
  });
}

/** web/src の .ts / .tsx（辞書ファイル・生成物・テストは除く）を読む */
function sources(dir: string, out: string[] = []): string[] {
  for (const name of readdirSync(dir)) {
    const p = join(dir, name);
    if (statSync(p).isDirectory()) {
      if (name !== "gen") sources(p, out);
    } else if (/\.tsx?$/.test(name) && !/\.test\.tsx?$/.test(name)) {
      if (!/\/i18n\/(.+\/)?(ja|en)\.ts$/.test(p)) out.push(p);
    }
  }
  return out;
}

/** 実行時に API の値などから組み立てて引く辞書の枝（プレフィックス）と、その理由 */
const DYNAMIC_PREFIXES: Record<string, string> = {
  "catalog.groupings.": "CatalogPage が CATALOG_GROUPINGS の値で引く",
  "cli.change.": "OpPreview が API の変更の種別（created など）で引く",
  "fixes.pick.action.": "analysis/MissingFixActions が API の選ぶものの種類（RefKind）で引く",
  "fixes.pick.prompt.": "MissingPickDialog が API の選ぶものの種類（RefKind）で引く",
  "fixes.pick.label.": "MissingPickDialog が API の選ぶものの種類（RefKind）で引く",
  "fixes.pick.none.": "MissingPickDialog が API の選ぶものの種類（RefKind）で引く",
  "fixes.addItem.": "analysis/MissingFixActions が API の一覧の種別（ListKind）で引く",
  "fields.labels.": "analysis/fields.ts が API の出力項目のキーで引く",
  "nav.items.": "lib/nav.ts が項目のキーで引く",
  "nav.sections.": "サイドバーが区分の id で引く",
  "maneuvers.targetKeys.": "ManeuversTab が API の目標のキーで引く",
  "verdict.": "VerdictPill などが API の判定で引く",
  "status.": "AnalysisTable が API の状態で引く",
  "requirements.reasons.": "ComplianceTab が API の理由で引く",
  "systems.aocsModes.fields.": "Attitude が API の出力項目のキーで引く",
  "systems.findings.kind.": "GeometryFindings が API の所見の種類（FindingKind）で引く",
  "systems.layout3d.helper.": "HelperToggles が補助表示のキーで引く",
  "project.data.ephemeris.": "DataPanel が API の天体暦の種別で引く",
  "diagram.toolbar.": "DiagramToolbar が層の名前（power / signal / gnd）で引く",
  "catalog.shared.hint.": "SharedCatalogsDialog が共有カタログの状態で引く",
  "systems.rangeChart.kinds.": "RangeTip などが温度範囲の種別で引く",
  "project.groups.": "SettingsPage が設定の区分（scope）で引く",
  "diagram.right": "RouteControls が接続の側（left / right / top / bottom）で引く",
  "diagram.bottom": "RouteControls が接続の側（left / right / top / bottom）で引く",
  "sources.note": "SourceDialog が欄の名前（LAYOUT の field）で引く",
  "systems.status.": "システムの状態を API の値で引く",
  "systems.kpiFill.": "OverviewPage が API の fill.basis で引く",
  "systems.specs.": "SystemPage がサブシステムの id で引く",
  "systems.tabs.": "SystemPage がタブのキーで引く",
};

describe("辞書の未使用キー", () => {
  it("すべての葉が web/src のどこかで参照されている", () => {
    const text = sources(join(import.meta.dirname, "..")).map((f) => readFileSync(f, "utf8"));
    const unused = leaves(dictionaries.ja as Shape).filter((path) => {
      if (Object.keys(DYNAMIC_PREFIXES).some((p) => path.startsWith(p))) return false;
      return !isReferenced(text, path.split("."));
    });
    expect(unused, `未使用の辞書キー:\n${unused.join("\n")}`).toEqual([]);
  });
});

/**
 * 親と葉が続けて書かれている（t.a.b）か、別名（`const td = t.dialog`）や分割代入で
 * 親の名前を含むファイルの中に `.葉` が現れる
 */
function isReferenced(files: string[], segs: string[]): boolean {
  const last = segs[segs.length - 1];
  const parent = segs[segs.length - 2];
  const access = new RegExp(`\\.${last}\\b|\\{[^}]*\\b${last}\\b[^}]*\\}\\s*=`);
  if (parent === undefined)
    return files.some((f) => access.test(f) || new RegExp(`\\.${last}\\b`).test(f));
  const parentWord = new RegExp(`\\.${parent}\\b`);
  return files.some((f) => parentWord.test(f) && access.test(f));
}
