import {
  CartesianGrid,
  Legend,
  Line,
  LineChart,
  ReferenceArea,
  ReferenceLine,
  ResponsiveContainer,
  Tooltip,
  XAxis,
  YAxis,
} from "recharts";
import type {
  DisplayChannel as Channel,
  PowerChartView,
  DisplaySeries as Series,
} from "@/api/types";
import { localized, useT } from "@/i18n";
import { toneAt, toneHex } from "@/lib/tone";
import { NumText, numText } from "@/quantity/chart";

/** 標本を表示だけ間引く。表示値は API の display_scale を掛ける。 */
/** 同じグラフの線すべての絶対値の最大（表示単位）。ツールチップで丸めの残り（10⁻¹⁶ など）を 0 と出すのに使う */
function peakOf(channels: readonly Channel[]): number {
  return channels.reduce(
    (m, c) => c.values.reduce((n, v) => Math.max(n, Math.abs(v * c.display_scale)), m),
    0,
  );
}

function chartData(series: Series, channels: readonly Channel[]) {
  const step = Math.max(1, Math.ceil(series.t.length / 1500));
  const data: Record<string, number>[] = [];
  for (let i = 0; i < series.t.length; i += step) {
    const row: Record<string, number> = { t: series.t[i] ?? 0 };
    for (const c of channels) {
      const v = c.values[i];
      if (v !== undefined) row[c.key] = v * c.display_scale;
    }
    data.push(row);
  }
  return data;
}

/** 系列を API の表示単位ごとに分ける。横軸は元期からの経過秒。 */
export function SeriesChart({
  series,
  title,
  timeline,
  syncId,
}: {
  series: Series;
  title?: string;
  timeline?: PowerChartView;
  syncId?: string;
}) {
  const tr = useT();
  const groups = new Map<string, Channel[]>();
  for (const c of series.channels)
    groups.set(c.display_unit, [...(groups.get(c.display_unit) ?? []), c]);
  return (
    <div className="space-y-4">
      {[...groups.entries()].map(([unit, channels]) => (
        <figure key={unit} className="m-0">
          <figcaption className="mb-1 text-ink-3 text-xs">
            {title ? `${title} · ` : ""}
            {/* 2 本以上なら名前は凡例に出るので、見出しには繰り返さない */}
            {channels.length === 1 &&
              channels[0] &&
              localized(channels[0].label, channels[0].label_en)}
            {tr.systems.seriesUnit(unit || "—")}
          </figcaption>
          <SeriesPlot
            series={series}
            channels={channels}
            unit={unit}
            timeline={timeline}
            syncId={syncId}
          />
        </figure>
      ))}
    </div>
  );
}

function SeriesPlot({
  series,
  channels,
  unit,
  timeline,
  syncId,
}: {
  series: Series;
  channels: readonly Channel[];
  unit: string;
  timeline?: PowerChartView;
  syncId?: string;
}) {
  const tr = useT();
  const peak = peakOf(channels);
  return (
    <div className="h-56">
      <ResponsiveContainer width="100%" height="100%">
        <LineChart
          data={chartData(series, channels)}
          syncId={syncId}
          syncMethod="value"
          margin={{ top: 4, right: 12, bottom: 0, left: 0 }}
        >
          <CartesianGrid stroke="var(--color-line)" vertical={false} />
          {chartAxes(
            tr.systems.elapsed,
            timeline?.domain,
            timeline ? !timeline.show_time_axis : false,
          )}
          {timeline?.bands
            .filter((band) => band.eclipse)
            .map((band) => (
              <ReferenceArea
                key={`${band.start}:${band.eclipse}`}
                x1={band.start}
                x2={band.end}
                fill="var(--color-ink-4)"
                fillOpacity={0.15}
              />
            ))}
          {timeline?.reference !== null && timeline?.reference !== undefined && (
            <ReferenceLine
              y={timeline.reference}
              stroke="var(--color-warn)"
              strokeDasharray="4 3"
              label={
                timeline.reference_label
                  ? localized(timeline.reference_label.label, timeline.reference_label.label_en)
                  : undefined
              }
            />
          )}
          <Tooltip
            formatter={(v) =>
              typeof v === "number" ? (
                <>
                  <NumText value={v} peak={peak} /> {unit}
                </>
              ) : (
                String(v)
              )
            }
            labelFormatter={(v) => `t = ${typeof v === "number" ? numText(v) : String(v)} s`}
            contentStyle={{
              fontSize: 12,
              borderRadius: 9,
              borderColor: "var(--color-line-strong)",
            }}
          />
          {channels.length > 1 && <Legend wrapperStyle={{ fontSize: 11 }} />}
          {channels.map((c, i) => (
            <Line
              key={c.key}
              dataKey={c.key}
              name={localized(c.label, c.label_en)}
              dot={false}
              isAnimationActive={false}
              strokeWidth={1.5}
              stroke={toneHex[toneAt(i)]}
            />
          ))}
        </LineChart>
      </ResponsiveContainer>
    </div>
  );
}

function chartAxes(elapsed: string, domain?: [number, number], hide = false) {
  return [
    <XAxis
      key="t"
      hide={hide}
      dataKey="t"
      type="number"
      domain={domain ?? ["dataMin", "dataMax"]}
      tickFormatter={(v: number) => numText(v)}
      tick={{ fontSize: 11, fill: "var(--color-ink-3)" }}
      label={{
        value: elapsed,
        position: "insideBottomRight",
        fontSize: 10,
        fill: "var(--color-ink-4)",
      }}
    />,
    <YAxis
      key="value"
      tickFormatter={(v: number) => numText(v)}
      tick={{ fontSize: 11, fill: "var(--color-ink-3)" }}
      width={56}
    />,
  ];
}
