import type { AnalysisRunResult, DisplaySeries as Series } from "@/api/types";

/**
 * 解析の出力（JSON、量は SI の数値）を画面の部品に分ける汎用の読み取り。
 * 解析ごとの形は sat-analysis の型（`web/src/api/gen/` に生成）。ここは形に依らず、
 * スカラー・時系列（Series）・表（オブジェクトの配列）を見つけるだけ。
 * 出力の中の位置は走査しながら区間の列（`JsonPath`）で持ち、JSON Pointer は `jsonPointer` だけが作る（文字列のパスを分解しない）。
 */
export type Json = string | number | boolean | null | Json[] | { [k: string]: Json };
export type JsonObject = { [k: string]: Json };

/** 出力の中の位置: キーと配列の番号の列（`["cases", 0, "max_dod"]`） */
export type JsonPath = readonly (string | number)[];

/**
 * 位置 → JSON Pointer（RFC 6901。`/cases/0/max_dod`、`~` は `~0`・`/` は `~1`）。**pointer を作るのはここだけ**。
 * API の dims のキーと同じ書き方（fune_service の analysis::dims）
 */
export function jsonPointer(path: JsonPath): string {
  return path.map((s) => `/${String(s).replaceAll("~", "~0").replaceAll("/", "~1")}`).join("");
}

/** 配列の全要素を表す区間（`FieldPath` の中で使う） */
export const EACH: unique symbol = Symbol("each");

/** 出力の欄の指し方（キーの列。`EACH` は配列の全要素）。系のページの KPI（systems.ts）が書く */
export type FieldPath = readonly (string | typeof EACH)[];

export function isObject(v: unknown): v is JsonObject {
  return typeof v === "object" && v !== null && !Array.isArray(v);
}

export function isSeries(v: unknown): v is Series {
  return isObject(v) && Array.isArray(v.t) && Array.isArray(v.channels);
}

/** `{x, y, z}`（Vec3）か */
export function isVec3(v: unknown): v is { x: number; y: number; z: number } {
  return (
    isObject(v) &&
    typeof v.x === "number" &&
    typeof v.y === "number" &&
    typeof v.z === "number" &&
    Object.keys(v).length === 3
  );
}

/** `{m: [[..],[..],[..]]}`（Mat3）か */
export function isMat3(v: unknown): v is { m: number[][] } {
  return isObject(v) && Array.isArray(v.m) && v.m.length === 3;
}

/** 値として 1 セルに入るもの（数・文字・真偽・Vec3・Mat3） */
export function isLeaf(v: unknown): boolean {
  return v === null || typeof v !== "object" || isVec3(v) || isMat3(v);
}

export interface FoundSeries {
  /** 表示用の位置（`cases[worst].soc`。配列の要素は id か番号） */
  path: string;
  series: Series;
}

/** 出力のどこかにある Series をすべて集める（表示用の位置付き） */
export function collectSeries(v: unknown, path = "", out: FoundSeries[] = []): FoundSeries[] {
  if (isSeries(v)) {
    out.push({ path, series: v });
  } else if (Array.isArray(v)) {
    v.forEach((x, i) => {
      const id = isObject(x) && typeof x.id === "string" ? x.id : String(i);
      collectSeries(x, `${path}[${id}]`, out);
    });
  } else if (isObject(v)) {
    for (const [k, x] of Object.entries(v)) collectSeries(x, path ? `${path}.${k}` : k, out);
  }
  return out;
}

export interface ScalarField {
  /** 出力の中の位置（配列の外なのでキーだけ） */
  at: string[];
  /** 表示用の位置（`summary.total_mass`） */
  path: string;
  /** JSON Pointer（`/summary/total_mass`）。API の dims（次元）を引くのに使う */
  pointer: string;
  key: string;
  value: Json;
}

/** 配列の外にあるスカラー（入れ子のオブジェクトの中も）。判定と時系列は除く */
export function collectScalars(
  v: unknown,
  at: readonly string[] = [],
  out: ScalarField[] = [],
): ScalarField[] {
  if (!isObject(v)) return out;
  for (const [k, x] of Object.entries(v)) {
    const here = [...at, k];
    if (k === "checks" || isSeries(x)) continue;
    if (isLeaf(x))
      out.push({ at: here, path: here.join("."), pointer: jsonPointer(here), key: k, value: x });
    else if (isObject(x)) collectScalars(x, here, out);
  }
  return out;
}

/** スカラーを入れ子のオブジェクトごと（`mtq.time` → mtq）に分ける。最上位は "" */
export function groupScalars(scalars: readonly ScalarField[]): [string, ScalarField[]][] {
  const m = new Map<string, ScalarField[]>();
  for (const s of scalars) {
    const g = s.at.length > 1 ? (s.at[0] ?? "") : "";
    m.set(g, [...(m.get(g) ?? []), s]);
  }
  return [...m.entries()].sort(([a], [b]) => (a === "" ? -1 : b === "" ? 1 : 0));
}

export interface FoundTable {
  /** 出力の中の位置（`["cases"]`）。セルの次元は `jsonPointer([...at, 行, 列])` で引く */
  at: string[];
  /** 表示用の位置（`cases`） */
  path: string;
  key: string;
  rows: JsonObject[];
}

/** 配列の外にある「オブジェクトの配列」（表にするもの）。checks と Series は除く */
export function collectTables(
  v: unknown,
  at: readonly string[] = [],
  out: FoundTable[] = [],
): FoundTable[] {
  if (!isObject(v)) return out;
  for (const [k, x] of Object.entries(v)) {
    const here = [...at, k];
    if (k === "checks" || isSeries(x)) continue;
    if (Array.isArray(x) && x.length > 0 && x.every(isObject)) {
      out.push({ at: here, path: here.join("."), key: k, rows: x as JsonObject[] });
    } else if (isObject(x) && !isLeaf(x)) collectTables(x, here, out);
  }
  return out;
}

/** 表の列: 行のどれかに出てくる、セルに入るキー（出てきた順） */
export function tableColumns(rows: readonly JsonObject[]): string[] {
  const cols: string[] = [];
  const nested = new Set<string>();
  for (const r of rows.slice(0, 50)) {
    for (const [k, v] of Object.entries(r)) {
      if (Array.isArray(v) && v.some((x) => isObject(x) && !isVec3(x))) nested.add(k);
      if (!cols.includes(k) && (isLeaf(v) || Array.isArray(v))) cols.push(k);
    }
  }
  return cols.filter((k) => !nested.has(k) && k !== "checks");
}

/** 行の中の入れ子の表（オブジェクトの配列） */
export function nestedTables(row: JsonObject): [string, JsonObject[]][] {
  return Object.entries(row).flatMap(([k, v]) =>
    k !== "checks" && Array.isArray(v) && v.length > 0 && v.every(isObject)
      ? [[k, v as JsonObject[]] as [string, JsonObject[]]]
      : [],
  );
}

/** 欄の指し方に当たる値と、その位置（`EACH` は配列の番号に開く）。見つからないものは捨てる */
function lookup(v: unknown, path: FieldPath): { at: JsonPath; value: unknown }[] {
  let cur: { at: JsonPath; value: unknown }[] = [{ at: [], value: v }];
  for (const seg of path) {
    cur = cur.flatMap(({ at, value }) => {
      if (seg === EACH)
        return Array.isArray(value) ? value.map((x, i) => ({ at: [...at, i], value: x })) : [];
      return isObject(value) && value[seg] !== undefined
        ? [{ at: [...at, seg], value: value[seg] }]
        : [];
    });
  }
  return cur;
}

/** 欄の指し方（`["modes", EACH, "total"]`）で数を引く。見つからない・数でないものは捨てる */
export function pickNumbers(v: unknown, path: FieldPath): number[] {
  return lookup(v, path)
    .map((m) => m.value)
    .filter((x): x is number => typeof x === "number" && Number.isFinite(x));
}

/**
 * 解析の結果の dims（JSON Pointer → 次元）から、欄の指し方に当たる欄の次元を引く（最初に当たったもの）。
 * 出力を欄の指し方でたどり、当たった位置の pointer（`jsonPointer`）で dims を引く。
 */
export function dimOf(
  result: Pick<AnalysisRunResult, "output" | "dims"> | null | undefined,
  path: FieldPath,
): string | null {
  if (!result) return null;
  for (const { at } of lookup(result.output, path)) {
    const dim = result.dims[jsonPointer(at)];
    if (dim) return dim;
  }
  return null;
}
