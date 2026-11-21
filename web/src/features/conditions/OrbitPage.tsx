import { AnalysisCard } from "@/analysis/AnalysisCard";
import { AnalysisKpis } from "@/analysis/AnalysisKpis";
import { ResultTable } from "@/analysis/ResultTable";
import { analysisOutput } from "@/api/analysis-output";
import { useAnalyses, useConditionAnalyses, useDesign } from "@/api/queries";
import type { GroundStationView, SettingView } from "@/api/types";
import { SettingsTable } from "@/features/project/SettingsTable";
import { useT } from "@/i18n";
import { Empty } from "@/kit/feedback/Empty";
import { SkeletonPanel } from "@/kit/feedback/Loading";
import { QueryState } from "@/kit/feedback/QueryState";
import { Panel } from "@/kit/panel/Panel";
import { PanelHeader } from "@/kit/panel/PanelHeader";
import { TablePanel } from "@/kit/table/TablePanel";
import { PageBody } from "@/shell/PageBody";
import { PageHeader } from "@/shell/PageHeader";
import { DecayPanel } from "./DecayPanel";
import { EclipseSeasonPanel } from "./EclipseSeasonPanel";
import { AddStationButton, StationActions } from "./StationDialogs";

/** 前提条件 / 軌道と地上局: 前提条件の設定（API の `scope`）と、地上局ごとの可視（comm.visibility） */
export function OrbitPage() {
  const t = useT();
  const m = t.conditions;
  const design = useDesign();
  const [vis] = useAnalyses(["comm.visibility"], null);
  const d = design.data;
  const stations = analysisOutput(vis?.result, "comm.visibility")?.stations ?? [];
  return (
    <>
      <PageHeader
        eyebrow={m.eyebrow}
        title={m.orbit}
        subtitle={m.orbitSubtitle}
        actions={<AddStationButton />}
      />
      <PageBody>
        <QueryState
          query={design}
          className="flex flex-col gap-4"
          skeleton={
            <>
              <SkeletonPanel lines={6} />
              <SkeletonPanel lines={4} />
            </>
          }
        >
          {(d) => <ConditionsSettings settings={d.settings} stations={d.stations} />}
        </QueryState>
        <OrbitAnalyses />
        {stations.length > 0 && (
          <TablePanel title={m.visibilityPerStation} caption="comm.visibility">
            <ResultTable
              rows={stations}
              at={["stations"]}
              dims={vis?.result?.dims}
              labels={vis?.result?.labels}
            />
          </TablePanel>
        )}
        {vis && <AnalysisCard state={vis} entry={undefined} design={d} />}
      </PageBody>
    </>
  );
}

/** 前提条件の設定（API の `scope` が conditions のもの）を、まとまり（`group`）ごとの表に */
function ConditionsSettings({
  settings,
  stations,
}: {
  settings: readonly SettingView[];
  stations: readonly GroundStationView[];
}) {
  const m = useT().conditions;
  const conditions = settings.filter((s) => s.scope === "conditions");
  // 設定を 1 つも書いていない地上局もカードを出す（名前の変更・削除ができるように）
  const groups = [...new Set([...conditions.map((s) => s.group), ...stations.map((s) => s.group)])];
  if (groups.length === 0)
    return (
      <Panel>
        <PanelHeader title={m.conditionsSettings} caption={m.conditionsSettingsCaption} />
        <Empty>{m.noConditionsSettings}</Empty>
      </Panel>
    );
  return (
    <div className="grid gap-4 xl:grid-cols-2">
      {groups.map((g) => {
        const list = conditions.filter((s) => s.group === g);
        const label = list.find((s) => s.field === "label");
        // 地上局のまとまりには名前の変更と削除（API の `stations` の `group` で引く）
        const station = stations.find((s) => s.group === g);
        return (
          <TablePanel
            key={g}
            title={label?.display ?? station?.id ?? g}
            right={station && <StationActions station={station} />}
            caption={<span className="font-mono">{list[0]?.file ?? station?.file ?? g}</span>}
          >
            {list.length > 0 ? (
              <SettingsTable settings={list} grouped />
            ) : (
              <Empty>{m.stations.noValues}</Empty>
            )}
          </TablePanel>
        );
      })}
    </div>
  );
}

/** 軌道の話題の解析はカタログから引く。未登録なら何も表示しない。 */
function OrbitAnalyses() {
  const catalog = useConditionAnalyses("orbit");
  const states = useAnalyses(catalog.ids, null);
  const design = useDesign().data;
  if (catalog.entries.length === 0) return null;
  return (
    <>
      <AnalysisKpis entries={catalog.entries} states={states} loading={catalog.loading} />
      {states.map((state) => (
        <AnalysisCard
          key={state.id}
          state={state}
          entry={catalog.entries.find((e) => e.info.id === state.id)}
          design={design}
        >
          {state.id === "orbit.eclipse_season" && state.result?.status === "ok" && (
            <EclipseSeasonPanel result={state.result} design={design} />
          )}
          {state.id === "orbit.decay" && state.result?.status === "ok" && (
            <DecayPanel result={state.result} />
          )}
        </AnalysisCard>
      ))}
    </>
  );
}
