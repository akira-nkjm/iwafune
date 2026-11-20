import { cn } from "@/lib/cn";
import { PANEL_MOTION } from "@/lib/motion";

/** サイドバーの開閉の動き（幅・行の高さ）。時間と曲線は画面の開け閉めで共通（lib/motion.ts） */
export const SIDEBAR_MOTION = PANEL_MOTION;

/**
 * アイコンのある行の左右の余白。アイコンの位置（サイドバーの端から 24 px）は、開いていても畳んでも同じにする:
 * 開いているときは行の中の余白が 16 px、畳んだときは行を 8 px 内側に寄せて余白を 8 px（32 px 四方の枠になる）。
 * 2 つの値は同時に切り替える（滑らかに変えると、途中でアイコンが揺れる）。`border` は枠線 1 px のある行（検索）。
 */
export function railItem(rail: boolean, border = false): string {
  if (border) return rail ? "mx-2 px-[7px]" : "px-[15px]";
  return rail ? "mx-2 px-2" : "px-4";
}

/** レール（畳んだ状態）で消える名前・キーの表示。幅が縮む間に薄くなり、開くときは幅が広がってから現れる */
export function railFade(rail: boolean): string {
  return cn(
    "whitespace-nowrap transition-opacity motion-reduce:transition-none",
    rail ? "pointer-events-none opacity-0 duration-100" : "opacity-100 delay-100 duration-200",
  );
}
