import type { ReactNode } from "react";
import type { AnalysisState } from "@/api/queries";
import type { AnalysisEntry, DesignView } from "@/api/types";
import { useT } from "@/i18n";
import { analysisDescription, analysisTitle } from "@/i18n/analysis-names";
import { Loading, SkeletonLines } from "@/kit/feedback/Loading";
import { Refreshable } from "@/kit/feedback/Refreshable";
import { Panel } from "@/kit/panel/Panel";
import { PanelHeader } from "@/kit/panel/PanelHeader";
import { Pill } from "@/kit/status/Pill";
import { ChecksList } from "./ChecksList";
import { AnalysisDetail, hasDetailView } from "./detail/AnalysisDetail";
import { FieldValue } from "./FieldValue";
import { fieldLabel } from "./fields";
import { MissingList } from "./MissingList";
import { collectScalars, collectTables, groupScalars } from "./output";
import { ResultTable } from "./ResultTable";

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
  const tables = output ? collectTables(output) : [];
  const checks = state.result?.checks ?? [];
  const title = analysisTitle(entry?.info, state.id);
  return (
    <Panel>
      <Refreshable active={state.updating}>
        <PanelHeader
          title={title}
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
          <Loading>
            <SkeletonLines lines={3} />
          </Loading>
        ) : state.error ? (
          <p className="text-neg-ink text-xs">{state.error}</p>
        ) : r ? (
          <div className="space-y-3">
            {r.error && <p className="text-neg-ink text-xs">{r.error}</p>}
            <MissingList design={design} missing={r.missing} />
            {/* 図（メッシュ上の場など）は、長くなりがちな判定の一覧より先に出す */}
            {entry?.detail_type !== undefined && hasDetailView(entry.detail_type, r) && (
              <AnalysisDetail result={r} detailType={entry.detail_type} />
            )}
            <ChecksList checks={checks} />
            {children}
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
                            {fieldLabel(s.key, s.at.at(-2))}
                          </dt>
                          <dd className="num truncate font-medium text-[13px] text-ink">
                            <FieldValue
                              field={s.key}
                              value={s.value}
                              dimension={r.dims[s.pointer]}
                              label={r.labels[s.pointer]}
                            />
                          </dd>
                        </div>
                      ))}
                    </dl>
                  </div>
                ))}
              </div>
            )}
            {tables.length > 0 && (
              <details>
                <summary className="cursor-pointer text-ink-3 text-xs">{t.systems.data}</summary>
                <div className="mt-3 space-y-4">
                  {tables.map((table) => (
                    <div key={table.path} className="overflow-auto">
                      <h3 className="mb-2 font-medium text-sm">{fieldLabel(table.key)}</h3>
                      <ResultTable
                        rows={table.rows}
                        at={table.at}
                        dims={r.dims}
                        labels={r.labels}
                      />
                    </div>
                  ))}
                </div>
              </details>
            )}
          </div>
        ) : null}
      </Refreshable>
    </Panel>
  );
}
