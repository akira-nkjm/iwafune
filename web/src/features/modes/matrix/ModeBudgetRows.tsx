import type { ModeHeader, ModeSummaryLabel, SystemModesView } from "@/api/types";
import { useT } from "@/i18n";
import { Loading, SkeletonLines } from "@/kit/feedback/Loading";
import { VerdictPill } from "@/kit/status/VerdictPill";
import { cn } from "@/lib/cn";
import { Formatted } from "@/quantity/Formatted";
import type { DraftComputed } from "../draft/use-mode-draft-load";
import { STICKY } from "./matrix";

/** 合計・収支。行（見出しの種類・読む欄・判定を添えるか）・値・判定は API が返し、ここは表示期間を選んで並べるだけ。 */
export function ModeBudgetRows({
  modes,
  view,
  eclipse,
}: {
  modes: ModeHeader[];
  view: DraftComputed<SystemModesView | null>;
  eclipse: boolean;
}) {
  const tm = useT().modes;
  const titles: Record<ModeSummaryLabel, string> = {
    power: tm.powerRow,
    data_rate: tm.dataRate,
    heat: tm.heat,
    available: tm.available,
    margin: tm.margin,
  };
  return (
    <>
      {(view.data?.summary ?? []).map((row) => (
        <tr key={row.label}>
          <th
            scope="row"
            className={cn(STICKY.rowHead, "bg-panel-2 px-4 py-3 text-left font-medium text-xs")}
          >
            {titles[row.label]}
          </th>
          <td />
          {modes.map((mode) => {
            const found = view.data?.modes.find((m) => m.id === mode.id);
            const values = eclipse ? found?.eclipse : found?.sunlit;
            const value = values?.[row.field];
            return (
              <td key={mode.id} className="num px-2 py-3 text-center text-xs">
                {view.pending ? (
                  <Loading>
                    <SkeletonLines lines={1} />
                  </Loading>
                ) : (
                  <>
                    {value ? (
                      <Formatted
                        si={value.si}
                        dimension={value.dimension}
                        fallback={value.display}
                      />
                    ) : (
                      "—"
                    )}
                    {row.verdict && values?.verdict && (
                      <span className="ml-2">
                        <VerdictPill verdict={values.verdict} />
                      </span>
                    )}
                  </>
                )}
              </td>
            );
          })}
        </tr>
      ))}
    </>
  );
}
