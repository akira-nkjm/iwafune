import type { ReactNode } from "react";
import type { BudgetRow, BudgetView, ModeHeader, ModeLoadOutput } from "@/api/types";
import { useT } from "@/i18n";
import { Loading, SkeletonLines } from "@/kit/feedback/Loading";
import { VerdictPill } from "@/kit/status/VerdictPill";
import { cn } from "@/lib/cn";
import { Formatted } from "@/quantity/Formatted";
import type { DraftComputed } from "../draft/use-mode-draft-load";
import { STICKY } from "./matrix";

/**
 * 表の足もとの 3 行: モードごとの合計（成長余裕込み）・使用可能電力（上限からシステム余裕を引いたもの）・
 * 余裕と判定。どれも電力のバジェット（`/budgets/power` の `modes`）の値で、集計と判定はサーバーが行う。
 * 下書きがあれば、それを書き込まずに当てた値（問い合わせは `useModeDraftLoad`。その間は前の値を薄く出す）。
 */
export function ModeBudgetRows({
  modes,
  budget,
  load,
  eclipse,
}: {
  modes: ModeHeader[];
  load: DraftComputed<ModeLoadOutput | null>;
  eclipse: boolean;
  budget: DraftComputed<BudgetView | undefined>;
}) {
  const t = useT();
  const tm = t.modes;
  const dim = budget.updating && "opacity-50";
  const cells = (className: string, render: (row: BudgetRow) => ReactNode) =>
    modes.map((mode) => {
      const row = budget.data?.modes.find((m) => m.node === mode.id);
      return (
        <td key={mode.id} className={className}>
          {budget.pending ? (
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
        {eclipse
          ? modes.map((m) => {
              const result = load.data?.modes.find((r) => r.id === m.id);
              return (
                <td
                  key={m.id}
                  className={cn(
                    "num border-line border-t px-2 py-3 text-center font-medium text-xs",
                    load.updating && "opacity-50",
                  )}
                >
                  {load.pending ? (
                    <Loading>
                      <SkeletonLines lines={1} />
                    </Loading>
                  ) : result ? (
                    <Formatted
                      si={result.total_eclipse_with_growth}
                      dimension="Power"
                      fallback="—"
                    />
                  ) : (
                    "—"
                  )}
                </td>
              );
            })
          : cells(
              cn("num border-line border-t px-2 py-3 text-center font-medium text-xs", dim),
              (r) => r.with_growth.display,
            )}
      </tr>
      <tr>
        <th scope="row" className={cn(head, "py-1")}>
          {tm.available}
        </th>
        <td />
        {cells(cn("num px-2 py-1 text-center text-ink-3 text-xs", dim), (r) =>
          eclipse ? "—" : (r.allocated?.display ?? "—"),
        )}
      </tr>
      <tr>
        <th scope="row" className={cn(head, "pt-2 pb-4")}>
          {tm.margin}
        </th>
        <td />
        {cells(cn("px-2 pt-2 pb-4 text-center transition-opacity", dim), (r) =>
          eclipse ? (
            "—"
          ) : (
            <VerdictPill
              verdict={r.verdict}
              text={r.margin ? `${t.verdict[r.verdict]} · ${r.margin.display}` : undefined}
            />
          ),
        )}
      </tr>
      {eclipse && (
        <tr>
          <td colSpan={modes.length + 2} className="px-4 pb-3 text-xs text-ink-3">
            {tm.eclipsePowerUnavailable}
          </td>
        </tr>
      )}
    </>
  );
}
