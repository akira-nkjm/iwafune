import type { ModeHeader, System, SystemModesView } from "@/api/types";
import { useT } from "@/i18n";
import { Loading, SkeletonLines } from "@/kit/feedback/Loading";
import { VerdictPill } from "@/kit/status/VerdictPill";
import { cn } from "@/lib/cn";
import { Formatted } from "@/quantity/Formatted";
import type { DraftComputed } from "../draft/use-mode-draft-load";
import { STICKY } from "./matrix";

/** 合計・収支。値と判定は API が返し、ここは表示期間を選んで並べるだけ。 */
export function ModeBudgetRows({
  modes,
  view,
  eclipse,
  system,
}: {
  modes: ModeHeader[];
  view: DraftComputed<SystemModesView | null>;
  eclipse: boolean;
  system: System;
}) {
  const t = useT();
  const tm = t.modes;
  const rows =
    system === "power" ? (["total", "available", "margin"] as const) : (["total"] as const);
  const title = system === "cdh" ? tm.dataRate : system === "thermal" ? tm.heat : tm.powerRow;
  return (
    <>
      {rows.map((key) => (
        <tr key={key}>
          <th
            scope="row"
            className={cn(STICKY.rowHead, "bg-panel-2 px-4 py-3 text-left font-medium text-xs")}
          >
            {key === "total" ? title : tm[key]}
          </th>
          <td />
          {modes.map((mode) => {
            const found = view.data?.modes.find((m) => m.id === mode.id);
            const values = eclipse ? found?.eclipse : found?.sunlit;
            const value = values?.[key];
            return (
              <td
                key={mode.id}
                className={cn("num px-2 py-3 text-center text-xs", view.updating && "opacity-50")}
              >
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
                    {(key === "margin" || (key === "total" && system !== "power")) &&
                      values?.verdict && (
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
