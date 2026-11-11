/**
 * README などに置くロゴの SVG（文字列）を、作図の正本（brand-art.ts）から組み立てる。
 * 3 種類（マークだけ・マーク＋名前・マーク＋名前＋説明文のバナー）× 地の明暗。ファビコンはマークだけの明るい版。
 * ファイルは `pnpm -C web brand` で書き出し、brand-svg.test.ts が書き出し済みのものと一致するかを検査する。
 */

import { brandHex, brandInkHex } from "@/lib/tone";
import { type Box, LOCKUP, MARK, MARK_BOX, TAGLINE, WORDMARK, wordmarkCenterY } from "./brand-art";

/** 地の明暗（明るい地・暗い地） */
export type BrandTheme = keyof typeof brandInkHex;

/** 書き出すファイル（リポジトリの根からのパス）と中身 */
export type BrandFile = { path: string; svg: () => string };

const NOTE =
  "<!-- web/src/shell/brand/brand-svg.ts が書き出したもの。手で直さず `pnpm -C web brand` で作り直す -->";

/** 位置と大きさは小数 2 桁、倍率は 5 桁まで */
const num = (v: number) => String(Math.round(v * 100) / 100);
const ratio = (v: number) => String(Math.round(v * 1e5) / 1e5);
const indent = (lines: string[]) => lines.map((l) => `  ${l}`);

/** マークの図形（MARK の座標のまま） */
function markShapes(theme: BrandTheme): string[] {
  return [
    `<g transform="translate(${MARK.shift.x} ${MARK.shift.y})">`,
    ...indent(MARK.faces.map((f) => `<path d="${f.d}" fill="${brandHex[f.fill]}"/>`)),
    `  <g stroke="${brandInkHex[theme].outline}" stroke-width="${MARK.strokeWidth}" stroke-linejoin="round" stroke-linecap="round">`,
    ...indent(indent([MARK.outline, ...MARK.ridges].map((d) => `<path d="${d}"/>`))),
    "  </g>",
    "</g>",
  ];
}

/** マークを、余白を除いた枠（MARK.tight）が `at` に重なるように置く */
function placedMark(theme: BrandTheme, at: Box): string[] {
  const t = MARK.tight;
  return [
    `<svg x="${num(at.x)}" y="${num(at.y)}" width="${num(at.w)}" height="${num(at.h)}" viewBox="${t.x} ${t.y} ${t.w} ${t.h}" fill="none">`,
    ...indent(markShapes(theme)),
    "</svg>",
  ];
}

/** 字形を、墨の左端が `x`、ベースラインが `baseline`、1 em が `em` になるように置く */
function placedGlyphs(
  paths: { d: string; fill: string }[],
  inkLeft: number,
  x: number,
  baseline: number,
  em: number,
): string[] {
  const s = em / 1000;
  return [
    `<g transform="translate(${num(x - inkLeft * s)} ${num(baseline)}) scale(${ratio(s)})">`,
    ...indent(paths.map((p) => `<path d="${p.d}" fill="${p.fill}"/>`)),
    "</g>",
  ];
}

function document(size: { w: number; h: number }, label: string, body: string[]): string {
  const w = num(size.w);
  const h = num(size.h);
  return [
    `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 ${w} ${h}" width="${w}" height="${h}" role="img" aria-label="${label}">`,
    `  ${NOTE}`,
    ...indent(body),
    "</svg>",
    "",
  ].join("\n");
}

const wordmarkPaths = (theme: BrandTheme) => [
  { d: WORDMARK.iwa, fill: brandInkHex[theme].iwa },
  { d: WORDMARK.fune, fill: brandInkHex[theme].fune },
];

/** マークだけ（正方形。面積の重心が中心に来るように MARK.shift で寄せてある） */
export function markSvg(theme: BrandTheme, size = 128): string {
  return document({ w: size, h: size }, "iwafune", [
    `<svg viewBox="0 0 ${MARK_BOX} ${MARK_BOX}" width="${size}" height="${size}" fill="none">`,
    ...indent(markShapes(theme)),
    "</svg>",
  ]);
}

/**
 * マーク＋名前。並べ方は LOCKUP（Web のサイドバーと同じ）: 名前の 1 em はマークの幅の半分、
 * マークの外枠の中心と名前の x ハイトの帯の中心を同じ高さに置く。
 */
export function logoSvg(theme: BrandTheme, markHeight = 48): string {
  const t = MARK.tight;
  const markWidth = (markHeight * t.w) / t.h;
  const em = LOCKUP.emPerMarkWidth * markWidth;
  const s = em / 1000;
  const pad = markHeight / 6;
  const center = pad + markHeight / 2;
  const nameLeft = pad + markWidth + LOCKUP.gapEm * em;
  const width = nameLeft + (WORDMARK.ink.right - WORDMARK.ink.left) * s + pad;
  return document({ w: width, h: markHeight + 2 * pad }, "iwafune", [
    ...placedMark(theme, { x: pad, y: pad, w: markWidth, h: markHeight }),
    ...placedGlyphs(
      wordmarkPaths(theme),
      WORDMARK.ink.left,
      nameLeft,
      center - wordmarkCenterY * s,
      em,
    ),
  ]);
}

/** バナーの寸法 [px]。README は幅 480 で表示する */
const BANNER = { w: 480, h: 130, nameEm: 64, taglineEm: 19, lineGap: 34 } as const;

/**
 * マーク＋名前＋説明文（README のバナー）。マークの上端を名前の墨の上端に、下端を説明文のベースラインに合わせ、
 * 名前と説明文は墨の左端をそろえる。全体を画面の中央に置く。
 */
export function bannerSvg(theme: BrandTheme): string {
  const t = MARK.tight;
  const { w, h, nameEm, taglineEm, lineGap } = BANNER;
  const nameAscent = (-WORDMARK.ink.top * nameEm) / 1000;
  const markHeight = nameAscent + lineGap;
  const markWidth = (markHeight * t.w) / t.h;
  const gap = LOCKUP.gapEm * nameEm;
  const nameWidth = ((WORDMARK.ink.right - WORDMARK.ink.left) * nameEm) / 1000;
  const taglineWidth = ((TAGLINE.ink.right - TAGLINE.ink.left) * taglineEm) / 1000;
  const left = (w - (markWidth + gap + Math.max(nameWidth, taglineWidth))) / 2;
  const top = (h - markHeight) / 2;
  const textLeft = left + markWidth + gap;
  const nameBaseline = top + nameAscent;
  const tagline = [{ d: TAGLINE.d, fill: brandInkHex[theme].tagline }];
  return document({ w, h }, "iwafune — Satellite design workbench", [
    ...placedMark(theme, { x: left, y: top, w: markWidth, h: markHeight }),
    ...placedGlyphs(wordmarkPaths(theme), WORDMARK.ink.left, textLeft, nameBaseline, nameEm),
    ...placedGlyphs(tagline, TAGLINE.ink.left, textLeft, nameBaseline + lineGap, taglineEm),
  ]);
}

/** 書き出すファイルの一覧 */
export const BRAND_FILES: readonly BrandFile[] = [
  { path: "web/public/favicon.svg", svg: () => markSvg("light") },
  ...(["light", "dark"] as const).flatMap((theme) => [
    { path: `docs/assets/mark-${theme}.svg`, svg: () => markSvg(theme) },
    { path: `docs/assets/logo-${theme}.svg`, svg: () => logoSvg(theme) },
    { path: `docs/assets/banner-${theme}.svg`, svg: () => bannerSvg(theme) },
  ]),
];
