/**
 * ロゴの作図の正本。マーク（天磐船の船体）、名前「iwafune」と説明文の字形、それらの並べ方。
 * 画面の部品（BrandMark・BrandWordmark）と、README などに置く SVG（brand-svg.ts が書き出す）はここから描く。
 *
 * マークは天磐船（饒速日命が乗って天降ったとされる磐の船）の船体を、斜め上から見た 3 つの面で切り出した図。
 *
 * 字形（brand-glyphs.ts）はフォントを輪郭にしたもの。README の SVG は GitHub が画像として表示し、見る人の
 * 端末のフォントで描かれてしまうので、字のままにしない。名前はヒラギノ角ゴシック W6、説明文は W4
 * （線の端がわずかに広がる字）。Mac の Core Text で字形をパスにし、字詰めもそのとき入れた。
 * ヒラギノは macOS 付属の商用フォントなので、フォントそのものはリポジトリに入れず、輪郭（図形）だけを置く。
 */

import { TAGLINE_GLYPHS, WORDMARK_GLYPHS } from "./brand-glyphs";

/** マークの viewBox の一辺。24 は lucide のアイコンと同じ土台。 */
export const MARK_BOX = 24;

/** マークの図形。面の色は lib/tone.ts の `brandHex` の名前で指す。 */
export const MARK = {
  faces: [
    /** 船首側の面（明るい水色） */
    { d: "M2.5 9.5H10.8L12.6 20.3L6.4 19.2L3.3 14.4Z", fill: "bow" },
    /** 甲板側の面（ACCENT1） */
    { d: "M10.8 9.5H21.5L18.6 16.4L11.6 14.3Z", fill: "deck" },
    /** 船尾下の影の面（DARK1） */
    { d: "M11.6 14.3L18.6 16.4L12.6 20.3Z", fill: "keel" },
  ],
  /** 外形と、面を分ける 2 本の稜線。 */
  outline: "M2.5 9.5H21.5L18.6 16.4L12.6 20.3L6.4 19.2L3.3 14.4Z",
  ridges: ["M10.8 9.5L12.6 20.3", "M11.6 14.3L18.6 16.4"],
  strokeWidth: 1.2,
  /**
   * 正方形の viewBox の中で光学的に中央へ寄せる量。外形の枠ではなく面積の重心（x≈11.5, y≈14.0）を
   * 中心（12, 12）に合わせる（ファビコンのような正方形の版で使う）。
   */
  shift: { x: 0.5, y: -2 },
  /**
   * 余白を除いた描画範囲（光学補正のあとの外形に、線の太さの半分を足したもの）。名前と並べる版はこの枠を使い、
   * 枠の中心を名前の x ハイトの帯の中心にそろえる。
   */
  tight: { x: 2.4, y: 6.9, w: 20.2, h: 12 },
} as const;

/** 名前「iwafune」の字形（1 em = 1000、ベースライン y = 0、y は下向き）。 */
export const WORDMARK = WORDMARK_GLYPHS;
/** 説明文「Satellite design workbench」の字形。 */
export const TAGLINE = TAGLINE_GLYPHS;

/**
 * マークと名前を横に並べるときの決まり。Web のサイドバーと README の SVG で共通。
 *
 * 縦は「マークの外枠（`MARK.tight`）の中心」と「名前の x ハイトの帯の中心」をそろえる。名前は小文字だけなので、
 * 見た目の中心は字の上下の端ではなく x ハイトの帯にある。マークは面積の重心ではなく外枠の中心で見る
 * （重心でそろえると、船の上の広い甲板に引かれて名前が高く見えた）。
 */
export const LOCKUP = {
  /** 名前の 1 em をマークの幅の何倍にするか（サイドバー: マーク 32 px・名前 16 px） */
  emPerMarkWidth: 0.5,
  /** マークの右端と名前の墨の左端の間隔 [em] */
  gapEm: 0.55,
} as const;

/** 矩形（viewBox の形）。 */
export type Box = { x: number; y: number; w: number; h: number };

/** 名前の x ハイトの帯の中心の y（ベースラインより上なので負）。 */
export const wordmarkCenterY = -WORDMARK.xHeight / 2;

/**
 * 名前だけを描くときの viewBox。横は墨の範囲、縦は x ハイトの帯の中心で上下対称に取る。
 * これで、外枠の中心が viewBox の中心にあるマークと flex の items-center で並べるだけで中心がそろう。
 */
export function wordmarkBox(): Box {
  const { ink } = WORDMARK;
  const half = Math.max(wordmarkCenterY - ink.top, ink.bottom - wordmarkCenterY);
  return { x: ink.left, y: wordmarkCenterY - half, w: ink.right - ink.left, h: 2 * half };
}
