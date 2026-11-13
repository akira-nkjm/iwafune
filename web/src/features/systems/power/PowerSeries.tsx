import { lazy, Suspense, useState } from "react";
import type { PowerPageView } from "@/api/types";
import { localized, useT } from "@/i18n";
import { Empty } from "@/kit/feedback/Empty";
import { Loading, SkeletonBlock } from "@/kit/feedback/Loading";
import { Panel } from "@/kit/panel/Panel";
import { PanelHeader } from "@/kit/panel/PanelHeader";
import { PowerControls } from "./PowerControls";
import { PowerOutline } from "./PowerOutline";

const SeriesChart = lazy(() =>
  import("@/analysis/SeriesChart").then((m) => ({ default: m.SeriesChart })),
);

/** API のグラフ定義を選択モードだけ描く。全チャートで時間・ホバーを共有する。 */
export function PowerSeries({ page }: { page: PowerPageView }) {
  const t = useT();
  const [selected, setSelected] = useState<string>();
  const modes = page.modes;
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
  charts: PowerPageView["modes"][number]["charts"];
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
