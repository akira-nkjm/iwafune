import { lazy, type ReactNode, Suspense } from "react";
import { AnalysisCard } from "@/analysis/AnalysisCard";
import { collectSeries } from "@/analysis/output";
import type { AnalysisState } from "@/api/queries";
import type { AnalysisEntry, DefsView, DesignView, System } from "@/api/types";
import { SystemModes } from "@/features/modes/matrix/ModesMatrix";
import { SettingsTable } from "@/features/project/SettingsTable";
import { useT } from "@/i18n";
import { analysisTitle } from "@/i18n/analysis-names";
import { Empty } from "@/kit/feedback/Empty";
import { Loading, SkeletonBlock } from "@/kit/feedback/Loading";
import { Settled } from "@/kit/feedback/Settled";
import { Panel } from "@/kit/panel/Panel";
import { PanelHeader } from "@/kit/panel/PanelHeader";
import { TablePanel } from "@/kit/table/TablePanel";
import { systemTitle } from "@/lib/design";
import { dedicatedViews } from "./dedicated";
import { hasExtraTab } from "./preload";
import { SystemOverview } from "./SystemOverview";

/** グラフ（recharts）は時系列のタブを開いたときだけ読む。系のページの最初のチャンクを小さくするため */
const SeriesChart = lazy(() =>
  import("@/analysis/SeriesChart").then((m) => ({ default: m.SeriesChart })),
);

export type SystemTab = "results" | "modes" | "heat" | "analyses" | "settings" | "series";

/**
 * 系のページのタブの中身（結果・モード・熱画像・設定・時系列）。extra は系ごとの追加の表示（熱の温度範囲の図と熱画像）。
 * 結果のタブは、熱は温度範囲の図だけ（解析のカードは解析結果のタブ）で、ほかの系は解析の表（`SystemOverview`）。
 */
export function SystemTabs({
  tab,
  kase,
  system,
  design,
  states,
  entries,
  extra,
  noAnalyses = false,
}: {
  tab: SystemTab;
  /** 選んだケース（電源のモード・時系列の問い合わせに使う） */
  kase: string | null;
  system: System;
  design: DesignView | undefined;
  states: AnalysisState[];
  entries: Map<string, AnalysisEntry>;
  extra?: Partial<Record<SystemTab, ReactNode>>;
  noAnalyses?: boolean;
}) {
  const dict = useT();
  // 専用の表示を持つ系（電源・AOCS）は登録表（`dedicated.tsx`）の表示。結果は解析の表（ほかの系と同じ）
  const dedicated = dedicatedViews(system);
  if (tab === "series" && dedicated?.series) return dedicated.series({ design, kase });
  if (tab === "modes" && dedicated?.modes) return dedicated.modes({ design, kase });
  if (tab === "modes")
    return design && hasModes(system, design.defs) ? (
      <SystemModes key={system} design={design} system={system} />
    ) : null;
  // 追加の表示（熱の図と熱画像）は、解析の結果と数字の表示がすべてそろってから一度に出す
  const settled = (node: ReactNode) => (
    <Settled
      pending={states.some((s) => s.loading)}
      label={dict.common.working.computing(systemTitle(system, design?.defs))}
    >
      {node}
    </Settled>
  );
  if (tab === "heat") return settled(extra?.heat);
  // 解析のカードの並び（判定・値・表）。このタブを持つ系（熱）は、概要には図だけを出す
  if (tab === "analyses")
    return (
      <div className="grid gap-4 xl:grid-cols-2">
        {states.map((s) => (
          <AnalysisCard key={s.id} state={s} entry={entries.get(s.id)} design={design} />
        ))}
      </div>
    );
  if (tab === "results")
    return (
      <>
        {noAnalyses && <Empty>{dict.systems.noAnalyses}</Empty>}
        {extra?.results && settled(extra.results)}
        {/* 解析結果のタブを持つ系（熱）の概要は図（`extra`）だけ。ほかの系は解析の表 */}
        {!hasExtraTab(system, "analyses") && (
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

/**
 * そのタブの中身が、ページの本文の高さいっぱいを使って自分の中でスクロールするか（部品 × モードの表）。
 * 表の見出しの行・左の列・下の合計を止めたままにするには、ページの本文ではなく表の中でスクロールさせる必要がある。
 * 専用のモードの見方を持つ系（`dedicated.tsx` の登録。AOCS のカード、電源の表）は、ふつうにページの本文がスクロールする。
 */
export function tabFillsBody(system: System, tab: SystemTab, defs: DefsView | undefined): boolean {
  return tab === "modes" && hasModes(system, defs) && !dedicatedViews(system)?.modes;
}

/** モードの見方を持つ系だけにタブを出す。どの系が持つかは API（`DefsView.mode_systems`）が返す。 */
export function hasModes(system: System, defs: DefsView | undefined): boolean {
  return defs?.mode_systems.includes(system) ?? false;
}
