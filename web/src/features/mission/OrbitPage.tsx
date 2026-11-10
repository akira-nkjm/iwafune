import { analysisOutput } from "@/api/analysis-output";
import { useDesign } from "@/api/queries";
import type { SettingView } from "@/api/types";
import { SettingsTable } from "@/features/project/SettingsTable";
import { AnalysisCard } from "@/features/systems/AnalysisCard";
import { ResultTable } from "@/features/systems/ResultTable";
import { useAnalyses } from "@/features/systems/use-analyses";
import { useT } from "@/i18n";
import { Empty } from "@/kit/Empty";
import { SkeletonPanel } from "@/kit/Loading";
import { Panel } from "@/kit/Panel";
import { PanelHeader } from "@/kit/PanelHeader";
import { QueryState } from "@/kit/QueryState";
import { TablePanel } from "@/kit/TablePanel";
import { PageBody } from "@/shell/PageBody";
import { PageHeader } from "@/shell/PageHeader";

/** ミッション / 軌道と地上局: ミッションの設定（API の `scope`）と、地上局ごとの可視（comm.visibility） */
export function OrbitPage() {
  const t = useT();
  const m = t.mission;
  const design = useDesign();
  const [vis] = useAnalyses(["comm.visibility"], null);
  const d = design.data;
  const stations = analysisOutput(vis?.result, "comm.visibility")?.stations ?? [];
  return (
    <>
      <PageHeader eyebrow={m.eyebrow} title={m.orbit} subtitle={m.orbitSubtitle} />
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
          {(d) => <MissionSettings settings={d.settings} />}
        </QueryState>
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

/** ミッションの設定（API の `scope` が mission のもの）を、まとまり（`group`）ごとの表に */
function MissionSettings({ settings }: { settings: readonly SettingView[] }) {
  const m = useT().mission;
  const mission = settings.filter((s) => s.scope === "mission");
  const groups = [...new Set(mission.map((s) => s.group))];
  if (groups.length === 0)
    return (
      <Panel>
        <PanelHeader title={m.missionSettings} caption={m.missionSettingsCaption} />
        <Empty>{m.noMissionSettings}</Empty>
      </Panel>
    );
  return (
    <div className="grid gap-4 xl:grid-cols-2">
      {groups.map((g) => {
        const list = mission.filter((s) => s.group === g);
        const label = list.find((s) => s.field === "label");
        return (
          <TablePanel
            key={g}
            title={label?.display ?? g}
            caption={<span className="font-mono">{list[0]?.file ?? g}</span>}
          >
            <SettingsTable settings={list} grouped />
          </TablePanel>
        );
      })}
    </div>
  );
}
