/**
 * 色の小さな計算（DOM を使わない純粋な関数）。配色の検査（WCAG のコントラスト・OKLab の差）と、色を混ぜる計算。
 * 色の値そのものは持たない（正本は `tone.ts`）。
 */

function channels(hex: string): [number, number, number] {
  const m = /^#?([0-9a-f]{6})$/i.exec(hex.trim());
  if (!m?.[1]) throw new Error(`bad hex ${hex}`);
  const n = Number.parseInt(m[1], 16);
  return [(n >> 16) & 255, (n >> 8) & 255, n & 255];
}

const lin = (c: number) => {
  const v = c / 255;
  return v <= 0.04045 ? v / 12.92 : ((v + 0.055) / 1.055) ** 2.4;
};

/** 線形の sRGB（0〜1）→ 8 bit の値（範囲の外は端に寄せる） */
const gamma = (v: number) => {
  const c = Math.max(0, Math.min(1, v));
  const s = c <= 0.0031308 ? 12.92 * c : 1.055 * c ** (1 / 2.4) - 0.055;
  return Math.round(s * 255);
};

/** WCAG relative luminance */
function luminance(hex: string): number {
  const [r, g, b] = channels(hex).map(lin) as [number, number, number];
  return 0.2126 * r + 0.7152 * g + 0.0722 * b;
}

/** WCAG contrast ratio (1..21) */
export function contrast(a: string, b: string): number {
  const x = luminance(a);
  const y = luminance(b);
  return (Math.max(x, y) + 0.05) / (Math.min(x, y) + 0.05);
}

function oklab(hex: string): [number, number, number] {
  const [r, g, b] = channels(hex).map(lin) as [number, number, number];
  const l = Math.cbrt(0.4122214708 * r + 0.5363325363 * g + 0.0514459929 * b);
  const m = Math.cbrt(0.2119034982 * r + 0.6806995451 * g + 0.1073969566 * b);
  const s = Math.cbrt(0.0883024619 * r + 0.2817188376 * g + 0.6299787005 * b);
  return [
    0.2104542553 * l + 0.793617785 * m - 0.0040720468 * s,
    1.9779984951 * l - 2.428592205 * m + 0.4505937099 * s,
    0.0259040371 * l + 0.7827717662 * m - 0.808675766 * s,
  ];
}

/** OKLab → `#rrggbb`（oklab の逆。sRGB の外は端に寄せる） */
function hexOfOklab([L, a, b]: [number, number, number]): string {
  const l = (L + 0.3963377774 * a + 0.2158037573 * b) ** 3;
  const m = (L - 0.1055613458 * a - 0.0638541728 * b) ** 3;
  const s = (L - 0.0894841775 * a - 1.291485548 * b) ** 3;
  const rgb = [
    4.0767416621 * l - 3.3077115913 * m + 0.2309699292 * s,
    -1.2684380046 * l + 2.6097574011 * m - 0.3413193965 * s,
    -0.0041960863 * l - 0.7034186147 * m + 1.707614701 * s,
  ];
  return `#${rgb.map((v) => gamma(v).toString(16).padStart(2, "0")).join("")}`;
}

/** Euclidean distance in OKLab (≈ perceptual difference; 0.02 is barely noticeable) */
export function deltaE(a: string, b: string): number {
  const x = oklab(a);
  const y = oklab(b);
  return Math.hypot(x[0] - y[0], x[1] - y[1], x[2] - y[2]);
}

/**
 * 2 色を OKLab で混ぜた `#rrggbb`（`t` = 0 で a、1 で b。範囲の外は端に寄せる）。CSS の `color-mix` で
 * `in oklab` を指定し b を t の割合で混ぜたときと同じ値になるので、CSS のクラスを使えない描画（three.js）とも色が揃う
 */
export function mixOklab(a: string, b: string, t: number): string {
  const r = Math.max(0, Math.min(1, t));
  const x = oklab(a);
  const y = oklab(b);
  return hexOfOklab([x[0] + (y[0] - x[0]) * r, x[1] + (y[1] - x[1]) * r, x[2] + (y[2] - x[2]) * r]);
}
