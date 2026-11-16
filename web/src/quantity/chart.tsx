/** 目盛りとツールチップの数（桁を揃えるだけ。単位と倍率は API の Channel.display_unit / display_scale） */
const sig = new Intl.NumberFormat(undefined, { maximumSignificantDigits: 4 });
const mantissa = new Intl.NumberFormat(undefined, { maximumSignificantDigits: 3 });

/** 数の表し方: 0・ふつう（有効数字 4 桁）・指数（仮数と 10 の指数） */
type NumParts =
  | { kind: "zero" }
  | { kind: "plain"; text: string }
  | { kind: "exp"; mantissa: string; exponent: number };

/**
 * `peak` は同じグラフの値の絶対値の最大（表示単位）で、その 10⁻⁹ 以下は計算の丸めとみなして 0 にする。
 * 0.001 未満と 10⁶ 以上は指数にする。
 */
export function numParts(n: number, peak = 1): NumParts {
  const a = Math.abs(n);
  if (a === 0 || a <= Math.max(Math.abs(peak), 1e-300) * 1e-9) return { kind: "zero" };
  if (!Number.isFinite(n) || (a >= 1e-3 && a < 1e6)) return { kind: "plain", text: sig.format(n) };
  const exponent = Math.floor(Math.log10(a));
  return { kind: "exp", mantissa: mantissa.format(n / 10 ** exponent), exponent };
}

/** 文字だけで出すとき（目盛り。SVG の文字なので上付きにしない）: 「2.14e-4」 */
export function numText(n: number, peak = 1): string {
  const p = numParts(n, peak);
  if (p.kind === "zero") return "0";
  if (p.kind === "plain") return p.text;
  return `${p.mantissa}e${p.exponent}`;
}

/** HTML で出すとき（ツールチップ）: 指数は上付きの要素で「2.14 × 10⁻⁴」 */
export function NumText({ value, peak = 1 }: { value: number; peak?: number }) {
  const p = numParts(value, peak);
  if (p.kind === "zero") return <>0</>;
  if (p.kind === "plain") return <>{p.text}</>;
  return (
    <span className="whitespace-nowrap">
      {p.mantissa} × 10<sup className="ml-px text-[0.75em]">{p.exponent}</sup>
    </span>
  );
}
