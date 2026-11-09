import { analysisOutput } from "@/api/analysis-output";
import { useDesign } from "@/api/queries";
import { SettingsTable } from "@/features/project/SettingsTable";
import { AnalysisCard } from "@/features/systems/AnalysisCard";
import { ResultTable } from "@/features/systems/ResultTable";
import { useAnalyses } from "@/features/systems/use-analyses";
import { useT } from "@/i18n";
import { Empty } from "@/kit/Empty";
import { Loading, SkeletonPanel } from "@/kit/Loading";
import { Panel } from "@/kit/Panel";
import { PanelHeader } from "@/kit/PanelHeader";
import { PageBody } from "@/shell/PageBody";
import { PageHeader } from "@/shell/PageHeader";

/** ミッション / 軌道と地上局: ミッションの設定（API の `scope`）と、地上局ごとの可視（comm.visibility） */
export function OrbitPage() {
  const t = useT();
  const m = t.mission;
  const design = useDesign();
  const [vis] = useAnalyses(["comm.visibility"], null);
  const d = design.data;
  const mission = (d?.settings ?? []).filter((s) => s.scope === "mission");
  const groups = [...new Set(mission.map((s) => s.group))];
  const stations = analysisOutput(vis?.result, "comm.visibility")?.stations ?? [];
  return (
    <>
      <PageHeader eyebrow={m.eyebrow} title={m.orbit} subtitle={m.orbitSubtitle} />
      <PageBody>
        {!d ? (
          <Loading className="space-y-4">
            <SkeletonPanel lines={6} />
            <SkeletonPanel lines={4} />
          </Loading>
        ) : groups.length === 0 ? (
          <Panel>
            <PanelHeader title={m.missionSettings} caption={m.missionSettingsCaption} />
            <Empty>{m.noMissionSettings}</Empty>
          </Panel>
        ) : (
          <div className="grid gap-4 xl:grid-cols-2">
            {groups.map((g) => {
              const list = mission.filter((s) => s.group === g);
              const label = list.find((s) => s.field === "label");
              return (
                <Panel key={g} pad={false}>
                  <div className="p-4 pb-2">
                    <PanelHeader
                      className="mb-0"
                      title={label?.display ?? g}
                      caption={<span className="font-mono">{list[0]?.file ?? g}</span>}
                    />
                  </div>
                  <SettingsTable settings={list} grouped />
                </Panel>
              );
            })}
          </div>
        )}
        {stations.length > 0 && (
          <Panel pad={false}>
            <div className="p-4 pb-2">
              <PanelHeader
                className="mb-0"
                title={m.visibilityPerStation}
                caption="comm.visibility"
              />
            </div>
            <ResultTable rows={stations} at={["stations"]} dims={vis?.result?.dims} />
          </Panel>
        )}
        {vis && <AnalysisCard state={vis} entry={undefined} design={d} />}
      </PageBody>
    </>
  );
}
