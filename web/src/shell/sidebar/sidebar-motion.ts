import { cn } from "@/lib/cn";
import { PANEL_MOTION } from "@/lib/motion";

/** サイドバーの開閉の動き（幅・行の高さ）。時間と曲線は画面の開け閉めで共通（lib/motion.ts） */
export const SIDEBAR_MOTION = PANEL_MOTION;

/**
 * アイコンのある行の置き方。行（色の付く地）の左端はサイドバーの端から 16 px（ロゴの左端と同じ）、アイコンは 24 px で、
 * 開いていても畳んでも同じ。畳むと、行は 32 px 四方の枠になる。`border` は枠線 1 px のある行（検索）。
 */
export function railItem(border = false): string {
  return border ? "mx-2 px-[7px]" : "mx-2 px-2";
}

/** レール（畳んだ状態）で消える名前・キーの表示。幅が縮む間に薄くなり、開くときは幅が広がってから現れる */
export function railFade(rail: boolean): string {
  return cn(
    "whitespace-nowrap transition-opacity motion-reduce:transition-none",
    rail ? "pointer-events-none opacity-0 duration-100" : "opacity-100 delay-100 duration-200",
  );
}
