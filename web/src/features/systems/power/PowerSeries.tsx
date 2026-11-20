import { lazy, Suspense, useState } from "react";
import type { PowerModeView } from "@/api/types";
import { localized, useT } from "@/i18n";
import { Empty } from "@/kit/feedback/Empty";
import { Loading, SkeletonBlock, SkeletonPanel } from "@/kit/feedback/Loading";
import { QueryState } from "@/kit/feedback/QueryState";
import { Refreshable } from "@/kit/feedback/Refreshable";
import { Panel } from "@/kit/panel/Panel";
import { PanelHeader } from "@/kit/panel/PanelHeader";
import { PowerControls } from "./PowerControls";
import { PowerOutline } from "./PowerOutline";
import { powerModesOf, usePowerModes } from "./use-power-modes";

const SeriesChart = lazy(() =>
  import("@/analysis/SeriesChart").then((m) => ({ default: m.SeriesChart })),
);

/** 選んだケースの電源の時系列（問い合わせは `usePowerModes`。描くのは `PowerSeriesView`）。 */
export function PowerSeries({ kase }: { kase: string | null }) {
  const query = usePowerModes(kase);
  return (
    <QueryState query={query} skeleton={<SkeletonPanel lines={6} />}>
      {(view) => (
        <Refreshable active={query.isFetching}>
          <PowerSeriesView modes={powerModesOf(view)} />
        </Refreshable>
      )}
    </QueryState>
  );
}

/** API のグラフ定義を選択モードだけ描く。全チャートで時間・ホバーを共有する。 */
export function PowerSeriesView({ modes }: { modes: PowerModeView[] }) {
  const t = useT();
  const [selected, setSelected] = useState<string>();
  const mode = modes.find((m) => m.id === selected) ?? modes[0];
  const charts = mode?.charts ?? [];
  const items = charts.map((chart) => ({
    id: `power-chart-${chart.id}`,
    label: localized(chart.label.label, chart.label.label_en),
  }));
  return (
    <div className="space-y-5">
      <PowerControls modes={modes} selected={mode?.id} onSelect={setSelected} />
      <BandLegend charts={charts} eclipseLabel={t.modes.eclipseView} />
      {charts.length === 0 ? (
        <Empty>{t.systems.noSeries}</Empty>
      ) : (
        <PowerOutline items={items}>
          {charts.map((chart) => (
            <section key={chart.id} id={`power-chart-${chart.id}`} className="scroll-mt-6">
              <Panel>
                <PanelHeader title={localized(chart.label.label, chart.label.label_en)} />
                <Suspense
                  fallback={
                    <Loading>
                      <SkeletonBlock className="h-56 w-full" />
                    </Loading>
                  }
                >
                  <SeriesChart series={chart.series} timeline={chart} syncId="power-timeline" />
                </Suspense>
              </Panel>
            </section>
          ))}
        </PowerOutline>
      )}
    </div>
  );
}

/** グラフの帯の凡例（蝕だけ。モードは上の選択で 1 つに決まるので帯にしない） */
function BandLegend({
  charts,
  eclipseLabel,
}: {
  charts: PowerModeView["charts"];
  eclipseLabel: string;
}) {
  if (!charts.some((c) => c.bands.some((b) => b.eclipse))) return null;
  return (
    <div className="flex items-center gap-1.5 text-ink-3 text-xs">
      <span aria-hidden="true" className="inline-block size-2.5 rounded-sm bg-ink-4/40" />
      {eclipseLabel}
    </div>
  );
}
