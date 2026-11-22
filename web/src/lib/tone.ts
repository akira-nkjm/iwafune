/**
 * 画面の色の正本（色の値はこのファイルだけに書く。xtask が検査する）。
 * - `palette`: 地・線・字・アクセント・状態の色。`CAT`: カテゴリ色（サブシステム・系列の区別。塗りと文字用の ink）。
 *   src/styles/index.css の色と影の変数（@theme の --color-*・--shadow-*）は、ここから書き出す（`pnpm -C web tokens`。
 *   tone.test.ts が一致と、コントラスト（塗りは 3:1 以上、文字の ink は 4.5:1 以上）・見分けやすさを検査する）。
 * - Tailwind のクラスの対応（toneBg など）。Tailwind はソースに字面で出たクラスだけを生成するので、省略せずに書く。
 * - CSS のクラスを使えない描画（three.js・SVG・README のロゴ）が使う hex（toneHex・uiHex・brandHex）。
 * - 色の混ぜ方（サブシステム色の淡い地 `toneTint`・熱画像の配色 `heatHex`・覆い `inkVeil`）。`color-mix(` と色の名前
 *   （`white` など）はここの外に書かない（xtask の web_colors が検査する）。
 */
import { mixOklab } from "./color";

export const tones = [
  "blue",
  "gold",
  "teal",
  "navy",
  "rose",
  "red",
  "gray",
  "green",
  "orange",
  "violet",
  "magenta",
] as const;
export type Tone = (typeof tones)[number];

/** カテゴリ色: 塗り（fill）と、白地の文字に使う色（ink。4.5:1 以上） */
const CAT: Record<Tone, { fill: string; ink: string }> = {
  blue: { fill: "#3382c2", ink: "#1f63a0" },
  gold: { fill: "#e7bd41", ink: "#8a6400" },
  teal: { fill: "#0097a7", ink: "#006d79" },
  navy: { fill: "#062654", ink: "#062654" },
  rose: { fill: "#e29797", ink: "#a33c3c" },
  red: { fill: "#b92626", ink: "#b92626" },
  gray: { fill: "#b6bcc6", ink: "#5b6577" },
  green: { fill: "#3c9a50", ink: "#2a7438" },
  orange: { fill: "#d9701f", ink: "#9c4a0a" },
  violet: { fill: "#7a5cc2", ink: "#5f42a6" },
  magenta: { fill: "#b04a96", ink: "#8e2f78" },
};

/** 字の色（紺）。線と影はこの色に透明度を付けたもの */
const INK = CAT.navy.fill;

/** 色に透明度を付ける（`rgba(r, g, b, a)`） */
function alpha(hex: string, a: number): string {
  const n = Number.parseInt(hex.slice(1), 16);
  return `rgba(${(n >> 16) & 255}, ${(n >> 8) & 255}, ${n & 255}, ${a})`;
}

/**
 * 地・線・字・アクセント・状態の色（キーは CSS の --color-<キー> の名前）。アクセントと状態の色はカテゴリ色から取る
 * （アクセント = 青、良 = 青緑、注意 = 金、悪 = 赤）。warn の塗りは白地の文字に使わない（文字は *-ink）。
 */
export const palette = {
  // 面
  app: "#fbfcfe",
  panel: "#ffffff",
  "panel-2": "#f5f7fa",
  sunken: "#e7ebf2",
  chip: "#eceff5",
  "line-soft": alpha(INK, 0.07),
  line: alpha(INK, 0.1),
  "line-strong": alpha(INK, 0.14),
  tick: "#dce1ea",
  // 文字
  ink: INK,
  "ink-2": "#414141",
  "ink-3": CAT.gray.ink,
  "ink-4": "#8d8d8d",
  inverse: "#ffffff",
  // リンク・選択・フォーカス
  accent: CAT.blue.fill,
  "accent-ink": CAT.blue.ink,
  "accent-tint": "#e8f1f9",
  // 状態
  pos: CAT.teal.fill,
  "pos-bg": "#e3f4f6",
  "pos-ink": CAT.teal.ink,
  warn: CAT.gold.fill,
  "warn-bg": "#fbf3d9",
  "warn-ink": CAT.gold.ink,
  neg: CAT.red.fill,
  "neg-bg": "#fbe9e9",
  "neg-ink": CAT.red.ink,
} as const;

/** 影（CSS の --shadow-<キー>） */
const SHADOWS = {
  panel: `0 1px 2px ${alpha(INK, 0.04)}, 0 1px 1px ${alpha(INK, 0.02)}`,
  pop: `0 12px 32px ${alpha(INK, 0.16)}, 0 2px 6px ${alpha(INK, 0.06)}`,
} as const;

/** index.css の書き出す部分の目印 */
export const CSS_TOKENS_BEGIN =
  "/* colors:begin — lib/tone.ts から書き出す（pnpm -C web tokens）。手で書き換えない */";
export const CSS_TOKENS_END = "/* colors:end */";

/** index.css の @theme に入れる色と影の変数（目印の行を含む。字下げは 2 文字） */
export function cssColorTokens(): string {
  const lines = [
    ...Object.entries(palette).map(([k, v]) => `--color-${k}: ${v};`),
    ...tones.map((t) => `--color-cat-${t}: ${CAT[t].fill};`),
    ...tones.map((t) => `--color-cat-${t}-ink: ${CAT[t].ink};`),
    ...Object.entries(SHADOWS).map(([k, v]) => `--shadow-${k}: ${v};`),
  ];
  return [CSS_TOKENS_BEGIN, ...lines, CSS_TOKENS_END].map((l) => `  ${l}`).join("\n");
}

/** 塗りの色（gold・rose・gray は白地の文字に使わない。fillOnly） */
export const toneHex: Record<Tone, string> = Object.fromEntries(
  tones.map((t) => [t, CAT[t].fill]),
) as Record<Tone, string>;

/** 文字用の色（白地で 4.5:1 以上） */
export const toneInkHex: Record<Tone, string> = Object.fromEntries(
  tones.map((t) => [t, CAT[t].ink]),
) as Record<Tone, string>;

/** 画面の地・線・字のうち、CSS のクラスを使えない描画（three.js・SVG の stroke）が使う色 */
export const uiHex = {
  ink: palette.ink,
  ink3: palette["ink-3"],
  ink4: palette["ink-4"],
  tick: palette.tick,
  accent: palette.accent,
} as const;

/**
 * ロゴのマーク（shell/brand/brand-art.ts）の 3 面の色。ブランドの青（LIGHT ACCENT / ACCENT1 / DARK1）で固定し、
 * テーマにも地の明暗にも追従させない。
 */
export const brandHex = {
  bow: "#a9c9e6",
  deck: CAT.blue.fill,
  keel: CAT.navy.fill,
} as const;

/**
 * ロゴの線と字の色。マークの外形の線、名前の「iwa」（岩）と「fune」（船）、説明文。明るい地では
 * 外形と「iwa」を船底の紺、「fune」を甲板の青にする。暗い地の版（README を GitHub のダークテーマで見たとき）は
 * 紺が沈むので、外形と「iwa」を明るい灰に、「fune」を明るい青に替える。
 */
export const brandInkHex = {
  light: { outline: brandHex.keel, iwa: brandHex.keel, fune: brandHex.deck, tagline: uiHex.ink3 },
  dark: { outline: "#cfe0f1", iwa: "#e6edf3", fune: "#6aa9df", tagline: "#9198a1" },
} as const;

/** 塗りが文字にも使えるか（白地で 3:1 未満の色は塗り専用） */
export const fillOnly: readonly Tone[] = ["gold", "rose", "gray"];

export const toneBg: Record<Tone, string> = {
  blue: "bg-cat-blue",
  gold: "bg-cat-gold",
  teal: "bg-cat-teal",
  navy: "bg-cat-navy",
  rose: "bg-cat-rose",
  red: "bg-cat-red",
  gray: "bg-cat-gray",
  green: "bg-cat-green",
  orange: "bg-cat-orange",
  violet: "bg-cat-violet",
  magenta: "bg-cat-magenta",
};

export const toneText: Record<Tone, string> = {
  blue: "text-cat-blue-ink",
  gold: "text-cat-gold-ink",
  teal: "text-cat-teal-ink",
  navy: "text-cat-navy-ink",
  rose: "text-cat-rose-ink",
  red: "text-cat-red-ink",
  gray: "text-cat-gray-ink",
  green: "text-cat-green-ink",
  orange: "text-cat-orange-ink",
  violet: "text-cat-violet-ink",
  magenta: "text-cat-magenta-ink",
};

export const toneBorder: Record<Tone, string> = {
  blue: "border-cat-blue",
  gold: "border-cat-gold",
  teal: "border-cat-teal",
  navy: "border-cat-navy",
  rose: "border-cat-rose",
  red: "border-cat-red",
  gray: "border-cat-gray",
  green: "border-cat-green",
  orange: "border-cat-orange",
  violet: "border-cat-violet",
  magenta: "border-cat-magenta",
};

/** 系列の色を順番に選ぶ（グラフの線など） */
export function toneAt(i: number): Tone {
  return tones[((i % tones.length) + tones.length) % tones.length] ?? "blue";
}

/**
 * カテゴリ色の淡い地の濃さ（面に混ぜるカテゴリ色の割合 %）。系統図のサブシステムの枠（frame）< 部品の詳細の見出しと
 * 系統図の組立品（header）< 一覧のアイコンの丸（icon）< 系統図の部品のカード（card）
 */
const TINT = { frame: 6, header: 10, icon: 16, card: 18 } as const;
export type TintLevel = keyof typeof TINT;

/** カテゴリ色を面（panel）の色に混ぜた淡い地（CSS の値。sRGB で混ぜる）。サブシステム色の地はこれだけで作る */
export function toneTint(tone: Tone, level: TintLevel): string {
  return `color-mix(in srgb, ${toneHex[tone]} ${TINT[level]}%, var(--color-panel))`;
}

/** 字の色を薄く透かした覆い（CSS の値）。系統図のミニマップで、いま見えている範囲の外にかける */
export const inkVeil = "color-mix(in srgb, var(--color-ink) 8%, transparent)";

/** 熱画像の配色の節（低い温度から高い温度へ。サーモグラフィの「アイアン」に近い、暗い藍 → 紫 → 赤 → 橙 → 黄 → 淡黄） */
const THERMAL_STOPS = [
  "#10104a",
  "#5a1d8c",
  "#b5307a",
  "#e8532f",
  "#f6a623",
  "#fbe45c",
  "#fff9cf",
] as const;

/**
 * 熱の配色: 色の位置 `ratio`（0 = 低い温度、1 = 高い温度。範囲外は端に丸める）を熱画像風の色（hex）にする。
 * 節の間は OKLab で混ぜる。3D の箱（three.js は CSS の color-mix を読めない）も凡例も同じこの関数で塗るので、同じ温度は同じ色になる
 */
export function heatHex(ratio: number): string {
  const r = Math.min(1, Math.max(0, ratio));
  const x = r * (THERMAL_STOPS.length - 1);
  const i = Math.min(Math.floor(x), THERMAL_STOPS.length - 2);
  return mixOklab(
    THERMAL_STOPS[i] ?? THERMAL_STOPS[0],
    THERMAL_STOPS[i + 1] ?? THERMAL_STOPS[0],
    x - i,
  );
}
