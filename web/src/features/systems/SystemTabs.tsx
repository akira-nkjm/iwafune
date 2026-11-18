import { lazy, type ReactNode, Suspense } from "react";
import { AnalysisCard } from "@/analysis/AnalysisCard";
import { collectSeries } from "@/analysis/output";
import type { AnalysisState } from "@/api/queries";
import type { AnalysisEntry, DefsView, DesignView, PowerPageView, System } from "@/api/types";
import { SystemModes } from "@/features/modes/matrix/ModesMatrix";
import { SettingsTable } from "@/features/project/SettingsTable";
import { useT } from "@/i18n";
import { analysisTitle } from "@/i18n/analysis-names";
import { Empty } from "@/kit/feedback/Empty";
import { Loading, SkeletonBlock } from "@/kit/feedback/Loading";
import { Panel } from "@/kit/panel/Panel";
import { PanelHeader } from "@/kit/panel/PanelHeader";
import { TablePanel } from "@/kit/table/TablePanel";
import { systemTitle } from "@/lib/design";
import { AocsModes } from "./aocs/AocsModes";
import { PowerModes } from "./power/PowerModes";
import { PowerOverview } from "./power/PowerOverview";
import { PowerSeries } from "./power/PowerSeries";
import { SystemOverview } from "./SystemOverview";

/** グラフ（recharts）は時系列のタブを開いたときだけ読む。系のページの最初のチャンクを小さくするため */
const SeriesChart = lazy(() =>
  import("@/analysis/SeriesChart").then((m) => ({ default: m.SeriesChart })),
);

export type SystemTab = "results" | "modes" | "settings" | "series";

/**
 * 系のページのタブの中身（結果・モード・設定・時系列）。extra は系ごとの追加の表示（熱の温度表と 3D）。
 * 結果のタブは、熱だけ解析のカードの並びで、ほかの系は解析の表（電源は `PowerOverview`、ほかは `SystemOverview`）。
 */
export function SystemTabs({
  tab,
  powerPage,
  system,
  design,
  states,
  entries,
  extra,
  noAnalyses = false,
}: {
  tab: SystemTab;
  powerPage?: PowerPageView;
  system: System;
  design: DesignView | undefined;
  states: AnalysisState[];
  entries: Map<string, AnalysisEntry>;
  extra?: Partial<Record<SystemTab, ReactNode>>;
  noAnalyses?: boolean;
}) {
  const dict = useT();
  if (system === "power" && powerPage) {
    if (tab === "results") return <PowerOverview page={powerPage} design={design} />;
    if (tab === "series") return <PowerSeries page={powerPage} />;
    if (tab === "modes") return <PowerModes page={powerPage} />;
  }
  if (tab === "modes")
    return design && hasModes(system, design.defs) ? (
      system === "aocs" ? (
        <AocsModes design={design} />
      ) : (
        <SystemModes key={system} design={design} system={system} />
      )
    ) : null;
  if (tab === "results")
    return (
      <>
        {noAnalyses && <Empty>{dict.systems.noAnalyses}</Empty>}
        {extra?.results}
        {system === "thermal" ? (
          // 熱は節点温度の表・3D（`extra`）と解析のカードのまま
          <div className="grid gap-4 xl:grid-cols-2">
            {states.map((s) => (
              <AnalysisCard key={s.id} state={s} entry={entries.get(s.id)} design={design} />
            ))}
          </div>
        ) : (
          <SystemOverview states={states} entries={entries} design={design} />
        )}
      </>
    );
  if (tab === "settings") {
    // 系と設定の対応は API（`SettingView.system`。正本は fune_service の query::settings）
    const settings = (design?.settings ?? []).filter((x) => x.system === system);
    const files = [...new Set(settings.flatMap((x) => (x.file ? [x.file] : [])))];
    return (
      <>
        <TablePanel
          title={dict.systems.settingsTitle(systemTitle(system, design?.defs))}
          caption={files.length > 0 ? dict.systems.settingsCaption(files.join(" / ")) : undefined}
        >
          <SettingsTable settings={settings} empty={dict.systems.noSettings} />
        </TablePanel>
        {states.map((s) =>
          s.result?.input ? (
            <Panel key={s.id}>
              <details>
                <summary className="cursor-pointer font-medium text-ink text-md">
                  {dict.systems.inputOf(analysisTitle(entries.get(s.id)?.info, s.id))}
                  <span className="ml-2 font-mono text-ink-3 text-xs">
                    {entries.get(s.id)?.input_type}
                  </span>
                </summary>
                <pre className="mt-2 max-h-96 overflow-auto rounded-control bg-panel-2 p-3 font-mono text-[11.5px] text-ink-2">
                  {JSON.stringify(s.result.input, null, 2)}
                </pre>
              </details>
            </Panel>
          ) : null,
        )}
      </>
    );
  }
  const found = states.flatMap((s) =>
    s.result?.output ? collectSeries(s.result.output).map((f) => ({ id: s.id, ...f })) : [],
  );
  if (found.length === 0) return <Empty>{dict.systems.noSeries}</Empty>;
  return (
    <>
      {found.map((f) => (
        <Panel key={`${f.id}:${f.path}`}>
          <PanelHeader
            title={analysisTitle(entries.get(f.id)?.info, f.id)}
            caption={<span className="font-mono">{`${f.id} · ${f.path}`}</span>}
          />
          <Suspense
            fallback={
              <Loading>
                <SkeletonBlock className="h-56 w-full" />
              </Loading>
            }
          >
            <SeriesChart series={f.series} />
          </Suspense>
        </Panel>
      ))}
      {extra?.series}
    </>
  );
}

/** モードの見方を持つ系だけにタブを出す。どの系が持つかは API（`DefsView.mode_systems`）が返す。 */
export function hasModes(system: System, defs: DefsView | undefined): boolean {
  return defs?.mode_systems.includes(system) ?? false;
}
