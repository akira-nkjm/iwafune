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

/**
 * 文字列の 64 ビットのハッシュ（FNV-1a。16 進 16 桁）。
 * `crypto.subtle` は使わない: 安全なコンテキスト（HTTPS か localhost）でしか使えず、LAN の HTTP で開いた画面では
 * `undefined` になる。この鍵は結果を残す場所の見出しで、秘密を守るものではないので、暗号のハッシュは要らない。
 */
function fnv1a64(text: string): string {
  const prime = 0x100000001b3n;
  const mask = 0xffffffffffffffffn;
  let hash = 0xcbf29ce484222325n;
  for (let i = 0; i < text.length; i++) {
    hash ^= BigInt(text.charCodeAt(i));
    hash = (hash * prime) & mask;
  }
  return hash.toString(16).padStart(16, "0");
}

/**
 * 版 `version` で `full` を配置するときの結果の鍵（版・入力の長さ・64 ビットのハッシュ）。同じ入力と版なら同じ鍵。
 * 長さを入れるのは、ハッシュが偶然一致しても、長さの違う入力を同じ鍵にしないため。
 */
export async function keyOf(full: DiagramModel, version: number): Promise<string> {
  const text = `iwafune-layout/${version}\n${layoutInput(full)}`;
  return `${version}-${text.length.toString(16)}-${fnv1a64(text)}`;
}

/** 配置の結果を残す鍵（いまの版で） */
export const planKey = (full: DiagramModel): Promise<string> => keyOf(full, LAYOUT_VERSION);
