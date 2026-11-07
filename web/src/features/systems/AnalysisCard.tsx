import type { ReactNode } from "react";
import type { AnalysisEntry, DesignView } from "@/api/types";
import { useT } from "@/i18n";
import { analysisDescription, analysisTitle } from "@/i18n/analysis-names";
import { Panel } from "@/kit/Panel";
import { PanelHeader } from "@/kit/PanelHeader";
import { Pill } from "@/kit/Pill";
import { Skeleton } from "@/ui/skeleton";
import { ChecksList } from "./ChecksList";
import { FieldValue } from "./FieldValue";
import { fieldLabel } from "./fields";
import { MissingList } from "./MissingList";
import { collectScalars, groupScalars } from "./output";
import type { AnalysisState } from "./use-analyses";

const statusTone = { ok: "pos", missing_inputs: "warn", failed: "neg" } as const;

/** 解析 1 つの結果のカード: 状態・足りない入力・判定・主な値（配列の外のスカラー） */
export function AnalysisCard({
  state,
  entry,
  design,
  children,
}: {
  state: AnalysisState;
  entry: AnalysisEntry | undefined;
  design: DesignView | undefined;
  children?: ReactNode;
}) {
  const t = useT();
  const r = state.result;
  const output = r?.output;
  const scalars = output ? collectScalars(output) : [];
  const checks = state.result?.checks ?? [];
  return (
    <Panel>
      <PanelHeader
        title={analysisTitle(entry?.info, state.id)}
        caption={
          <>
            <span className="font-mono">{state.id}</span>
            {entry &&
              ` · ${entry.info.kind === "steady" ? t.systems.steady : t.systems.timeSeries} · ${analysisDescription(entry.info)}`}
          </>
        }
        right={
          state.loading ? (
            <Pill>{t.systems.status.running}</Pill>
          ) : state.error ? (
            <Pill tone="neg">{t.systems.status.error}</Pill>
          ) : r ? (
            <Pill tone={statusTone[r.status]}>{t.systems.status[r.status]}</Pill>
          ) : null
        }
      />
      {state.loading ? (
        <Skeleton className="h-16 w-full" />
      ) : state.error ? (
        <p className="text-neg-ink text-xs">{state.error}</p>
      ) : r ? (
        <div className="space-y-3">
          {r.error && <p className="text-neg-ink text-xs">{r.error}</p>}
          <MissingList design={design} missing={r.missing} />
          <ChecksList checks={checks} />
          {scalars.length > 0 && (
            <div className="space-y-3">
              {groupScalars(scalars).map(([group, list]) => (
                <div key={group || "top"}>
                  {group && (
                    <div className="mb-1 font-semibold text-[11.5px] text-ink-2">
                      {fieldLabel(group)}
                    </div>
                  )}
                  <dl className="grid grid-cols-[repeat(auto-fill,minmax(180px,1fr))] gap-x-4 gap-y-2">
                    {list.map((s) => (
                      <div key={s.path} className="min-w-0">
                        <dt className="truncate text-2xs text-ink-3" title={s.path}>
                          {fieldLabel(s.key)}
                        </dt>
                        <dd className="num truncate font-medium text-[13px] text-ink">
                          <FieldValue field={s.key} value={s.value} dimension={r.dims[s.pointer]} />
                        </dd>
                      </div>
                    ))}
                  </dl>
                </div>
              ))}
            </div>
          )}
          {children}
        </div>
      ) : null}
    </Panel>
  );
}
