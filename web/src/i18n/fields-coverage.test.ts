import { existsSync, readFileSync } from "node:fs";
import { join } from "node:path";
import { expect, it } from "vitest";
import { dictionaries } from "./index";

/**
 * 解析の出力の欄の表示名（`fields.labels`）に漏れが無いこと。汎用の結果の表（`analysis/ResultTable`・`AnalysisCard`）は
 * 欄のキーを `fieldLabel` で引き、辞書に無ければキーをそのまま（`burn_time`）見せるので、解析の出力の型（`api/gen/` に生成。
 * `AnalysisOutputs` が全解析の出力の型を並べる）をたどって欄のキーを集め、日英どちらの辞書にも無いものを報告する。
 * 解析が欄を足したら、このテストが落ちて辞書の追加を求める（手で書いた一覧は持たない）。
 */
const GEN = join(import.meta.dirname, "..", "api", "gen");

/** 汎用の画面が欄の名前として引かないもの: 判定（`checks`）・時系列（`series`）は別の部品、ベクトルと行列は 1 セルに出す */
const NOT_LABELLED_AS_FIELDS = new Set(["checks", "series", "x", "y", "z", "m"]);
/** 中身を欄として見ない型（判定・時系列・ベクトル・行列。別の部品が出す） */
const OPAQUE_TYPES = new Set(["Check", "Series", "Vec3", "Mat3", "Verdict"]);

function read(type: string): string | null {
  const file = join(GEN, `${type}.ts`);
  return existsSync(file) ? readFileSync(file, "utf8") : null;
}

/** 生成した型のフィールド名（コメントを除いた `名前:` と `名前?:`）と、参照する型 */
function fieldsOf(source: string): { keys: string[]; refs: string[] } {
  const code = source.replace(/\/\*[\s\S]*?\*\//g, "");
  const refs = [...code.matchAll(/import type \{ (\w+) \}/g)].map((m) => m[1] ?? "");
  const body = code.split("export type")[1] ?? "";
  const keys = [...body.matchAll(/(?:^|[{,]\s*)([a-z_][a-z0-9_]*)\??:/gm)].map((m) => m[1] ?? "");
  return { keys, refs };
}

/** 解析の出力の型から、たどれる型すべての欄のキー（欄 → それを持つ型） */
function outputFields(): Map<string, string> {
  const roots = [...(read("AnalysisOutputs") ?? "").matchAll(/": (\w+);/g)].map((m) => m[1] ?? "");
  const found = new Map<string, string>();
  const seen = new Set<string>();
  const visit = (type: string) => {
    if (seen.has(type) || OPAQUE_TYPES.has(type)) return;
    seen.add(type);
    const source = read(type);
    if (source === null) return;
    const { keys, refs } = fieldsOf(source);
    for (const k of keys) if (!found.has(k)) found.set(k, type);
    for (const r of refs) visit(r);
  };
  for (const r of roots) visit(r);
  return found;
}

it("解析の出力の欄はすべて日英の辞書に表示名がある", () => {
  const fields = outputFields();
  expect(fields.size, "出力の型をたどれていない").toBeGreaterThan(300);
  for (const lang of ["ja", "en"] as const) {
    const labels = dictionaries[lang].fields.labels;
    const missing = [...fields]
      .filter(([k]) => !NOT_LABELLED_AS_FIELDS.has(k) && labels[k] === undefined)
      .map(([k, type]) => `${k}（${type}）`);
    expect(missing, `${lang} の fields.labels に無い欄:\n${missing.join("\n")}`).toEqual([]);
  }
});

it("日本語と英語の辞書は同じ欄を持つ", () => {
  const ja = Object.keys(dictionaries.ja.fields.labels).sort();
  const en = Object.keys(dictionaries.en.fields.labels).sort();
  expect(en).toEqual(ja);
});
