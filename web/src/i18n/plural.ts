/**
 * 英語の個数つきの名詞（`1 part` / `3 parts`）。英語の辞書の個数の文はここを通す（`1 parts` を作らない）。
 * 不規則な複数形は `many` に書く。日本語は数詞が変わらないので使わない。
 */
export function plural(n: number, one: string, many = `${one}s`): string {
  return `${n} ${n === 1 ? one : many}`;
}

/** 個数に合わせた動詞の形（`1 needs` / `2 need`）。 */
export function verb(n: number, one: string, many: string): string {
  return n === 1 ? one : many;
}
