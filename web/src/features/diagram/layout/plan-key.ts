import type { DiagramModel } from "../model";

/**
 * 配置のアルゴリズムの版。`planLayout`（と、それが使う place・compact・ports・refine・route・score の寸法や費用）の
 * 結果が変わる変更をしたら、必ず上げる。結果を残す鍵（`planKey`）に入るので、上げると古い結果は使われない。
 * 上げ忘れは plan-key.test.ts が（デモの結果の写しとの照合で）見つける。
 */
export const LAYOUT_VERSION = 1;

/** オブジェクトのキーを並べ替えて JSON にする（同じ中身なら、作られ方に依らず同じ文字列） */
function stable(value: unknown): string {
  if (Array.isArray(value)) return `[${value.map(stable).join(",")}]`;
  if (value && typeof value === "object") {
    const entries = Object.entries(value as Record<string, unknown>)
      .filter(([, v]) => v !== undefined)
      .sort(([a], [b]) => (a < b ? -1 : a > b ? 1 : 0));
    return `{${entries.map(([k, v]) => `${JSON.stringify(k)}:${stable(v)}`).join(",")}}`;
  }
  return JSON.stringify(value) ?? "null";
}

/**
 * 配置が読む入力の文字列。配置は枠・カード・ジャンクション・線だけを読む（凡例の色は読まない）ので、凡例は外す。
 * `planLayout` に渡す設定は無いので、入力は表示モデルだけ。
 */
export function layoutInput(full: DiagramModel): string {
  const { legend: _legend, ...rest } = full;
  return stable(rest);
}

const hex = (bytes: ArrayBuffer) =>
  [...new Uint8Array(bytes)].map((b) => b.toString(16).padStart(2, "0")).join("");

/** 版 `version` で `full` を配置するときの結果の鍵（SHA-256 の 16 進）。同じ入力と版なら同じ鍵 */
export async function keyOf(full: DiagramModel, version: number): Promise<string> {
  const text = `iwafune-layout/${version}\n${layoutInput(full)}`;
  return hex(await crypto.subtle.digest("SHA-256", new TextEncoder().encode(text)));
}

/** 配置の結果を残す鍵（いまの版で） */
export const planKey = (full: DiagramModel): Promise<string> => keyOf(full, LAYOUT_VERSION);
