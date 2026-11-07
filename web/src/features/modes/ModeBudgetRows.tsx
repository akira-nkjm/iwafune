import { useQuery } from "@tanstack/react-query";
import type { ReactNode } from "react";
import { q } from "@/api/queries";
import type { BudgetRow, ModeHeader, Op } from "@/api/types";
import { useT } from "@/i18n";
import { Loading, SkeletonLines } from "@/kit/Loading";
import { VerdictPill } from "@/kit/VerdictPill";
import { cn } from "@/lib/cn";
import { STICKY } from "./matrix";

/**
 * 表の足もとの 3 行: モードごとの合計（成長余裕込み）・使用可能電力（上限からシステム余裕を引いたもの）・
 * 余裕と判定。どれも電力のバジェット（`/budgets/power` の `modes`）の値で、集計と判定はサーバーが行う。
 * 下書き（`ops`）があれば、それを書き込まずに当てた値（問い合わせの間は前の値を薄く出す）。
 */
export function ModeBudgetRows({ modes, ops }: { modes: ModeHeader[]; ops: readonly Op[] }) {
  const t = useT();
  const tm = t.modes;
  const budget = useQuery(q.budget("power", ops));
  const dim = budget.isFetching && !budget.isPending && "opacity-50";
  const cells = (className: string, render: (row: BudgetRow) => ReactNode) =>
    modes.map((mode) => {
      const row = budget.data?.modes.find((m) => m.node === mode.id);
      return (
        <td key={mode.id} className={className}>
          {budget.isPending ? (
            <Loading>
              <SkeletonLines lines={1} />
            </Loading>
          ) : row ? (
            render(row)
          ) : (
            "—"
          )}
        </td>
      );
    });
  // 行の名前は部品の列に置き、部品の列と同じく左に貼り付ける（足もとの地の色で塗る）。
  // 電力レベルの列は空けておく（2 列ぶんを貼り付けると、横に流したときに最初のモードの値が隠れる）
  const head = cn(STICKY.rowHead, "bg-panel-2 px-4 text-left font-normal text-ink-3 text-xs");
  return (
    <>
      <tr>
        <th scope="row" className={cn(head, "border-t py-3 font-medium text-ink")}>
          {tm.powerRow}
        </th>
        <td className="border-line border-t" />
        {cells(
          cn("num border-line border-t px-2 py-3 text-center font-medium text-xs", dim),
          (r) => r.with_growth.display,
        )}
      </tr>
      <tr>
        <th scope="row" className={cn(head, "py-1")}>
          {tm.available}
        </th>
        <td />
        {cells(
          cn("num px-2 py-1 text-center text-ink-3 text-xs", dim),
          (r) => r.allocated?.display ?? "—",
        )}
      </tr>
      <tr>
        <th scope="row" className={cn(head, "pt-2 pb-4")}>
          {tm.margin}
        </th>
        <td />
        {cells(cn("px-2 pt-2 pb-4 text-center transition-opacity", dim), (r) => (
          <VerdictPill
            verdict={r.verdict}
            text={r.margin ? `${t.verdict[r.verdict]} · ${r.margin.display}` : undefined}
          />
        ))}
      </tr>
    </>
  );
}
