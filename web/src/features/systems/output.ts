import type { DisplaySeries as Series } from "@/api/types";

/**
 * 解析の出力（JSON、量は SI の数値）を画面の部品に分ける汎用の読み取り。
 * 解析ごとの形は sat-analysis の型（`web/src/api/gen/` に生成）。ここは形に依らず、
 * スカラー・時系列（Series）・表（オブジェクトの配列）を見つけるだけ。
 */
export type Json = string | number | boolean | null | Json[] | { [k: string]: Json };
export type JsonObject = { [k: string]: Json };

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
  path: string;
  series: Series;
}

/** 出力のどこかにある Series をすべて集める（パス付き） */
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
  /** `summary.total_mass` のようなドット区切り */
  path: string;
  /** JSON Pointer（`/summary/total_mass`）。API の dims（次元）を引くのに使う */
  pointer: string;
  key: string;
  value: Json;
}

/** 配列の外にあるスカラー（入れ子のオブジェクトの中も）。判定と時系列は除く */
export function collectScalars(v: unknown, path = "", out: ScalarField[] = []): ScalarField[] {
  if (!isObject(v)) return out;
  for (const [k, x] of Object.entries(v)) {
    const p = path ? `${path}.${k}` : k;
    if (k === "checks" || isSeries(x)) continue;
    if (isLeaf(x)) out.push({ path: p, pointer: `/${p.replaceAll(".", "/")}`, key: k, value: x });
    else if (isObject(x)) collectScalars(x, p, out);
  }
  return out;
}

/** スカラーを入れ子のオブジェクトごと（`mtq.time` → mtq）に分ける。最上位は "" */
export function groupScalars(scalars: readonly ScalarField[]): [string, ScalarField[]][] {
  const m = new Map<string, ScalarField[]>();
  for (const s of scalars) {
    const i = s.path.indexOf(".");
    const g = i > 0 ? s.path.slice(0, i) : "";
    m.set(g, [...(m.get(g) ?? []), s]);
  }
  return [...m.entries()].sort(([a], [b]) => (a === "" ? -1 : b === "" ? 1 : 0));
}

export interface FoundTable {
  path: string;
  /** JSON Pointer（`/cases`） */
  pointer: string;
  key: string;
  rows: JsonObject[];
}

/** 配列の外にある「オブジェクトの配列」（表にするもの）。checks と Series は除く */
export function collectTables(v: unknown, path = "", out: FoundTable[] = []): FoundTable[] {
  if (!isObject(v)) return out;
  for (const [k, x] of Object.entries(v)) {
    const p = path ? `${path}.${k}` : k;
    if (k === "checks" || isSeries(x)) continue;
    if (Array.isArray(x) && x.length > 0 && x.every(isObject)) {
      out.push({ path: p, pointer: `/${p.replaceAll(".", "/")}`, key: k, rows: x as JsonObject[] });
    } else if (isObject(x) && !isLeaf(x)) collectTables(x, p, out);
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

/**
 * パスで数を引く。`modes[].total` のように `[]` を付けると配列の全要素から集める。
 * 見つからない・数でないものは捨てる。
 */
export function pickNumbers(v: unknown, path: string): number[] {
  const parts = path.split(".");
  let cur: unknown[] = [v];
  for (const raw of parts) {
    const many = raw.endsWith("[]");
    const key = many ? raw.slice(0, -2) : raw;
    const next: unknown[] = [];
    for (const c of cur) {
      if (!isObject(c)) continue;
      const x = c[key];
      if (many && Array.isArray(x)) next.push(...x);
      else if (!many && x !== undefined) next.push(x);
    }
    cur = next;
  }
  return cur.filter((x): x is number => typeof x === "number" && Number.isFinite(x));
}

/**
 * API の dims（JSON Pointer → 次元）から、パス（`cases[].max_dod`）に当たる欄の次元を引く（最初に当たったもの）。
 */
export function dimOf(dims: Record<string, string> | undefined, path: string): string | null {
  if (!dims) return null;
  const re = new RegExp(
    `^/${path
      .split(".")
      .map((seg) => (seg.endsWith("[]") ? `${seg.slice(0, -2)}/\\d+` : seg))
      .join("/")}$`,
  );
  for (const [k, v] of Object.entries(dims)) if (re.test(k)) return v;
  return null;
}
