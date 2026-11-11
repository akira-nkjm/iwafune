import { cn } from "@/lib/cn";
import { PANEL_MOTION } from "@/lib/motion";

/** サイドバーの開閉の動き（幅・行の高さ）。時間と曲線は画面の開け閉めで共通（lib/motion.ts） */
export const SIDEBAR_MOTION = PANEL_MOTION;

/** レール（畳んだ状態）で消える名前・キーの表示。幅が縮む間に薄くなり、開くときは幅が広がってから現れる */
export function railFade(rail: boolean): string {
  return cn(
    "whitespace-nowrap transition-opacity motion-reduce:transition-none",
    rail ? "pointer-events-none opacity-0 duration-100" : "opacity-100 delay-100 duration-200",
  );
}
