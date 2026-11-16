import type { ReactNode } from "react";
import { cn } from "@/lib/cn";

/** カード 1 枚の幅 [px] */
export const CARD_WIDTH = 216;

/** 枠の中のカードの列の数（1〜2 枚はその数、3〜6 枚は 2 列、それより多ければ 3 列） */
export const columnsFor = (n: number) => (n <= 2 ? Math.max(1, n) : n <= 6 ? 2 : 3);

/** カードを `columnsFor` の列数で並べる格子（`count` はカードの枚数）。ページが違っても同じ並びになる */
export function CardGrid({ count, children }: { count: number; children: ReactNode }) {
  return (
    <div
      className="grid gap-5"
      style={{ gridTemplateColumns: `repeat(${columnsFor(count)}, ${CARD_WIDTH}px)` }}
    >
      {children}
    </div>
  );
}

/** 枠の中の入れ子の枠の並び（折り返す）。`afterCards` はその上にカードの格子があるとき余白を空ける */
export function NestedBoxes({
  afterCards = false,
  children,
}: {
  afterCards?: boolean;
  children: ReactNode;
}) {
  return (
    <div className={cn("flex flex-wrap items-start gap-4", afterCards && "mt-4")}>{children}</div>
  );
}
