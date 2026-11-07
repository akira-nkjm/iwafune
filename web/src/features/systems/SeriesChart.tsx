import {
  CartesianGrid,
  Legend,
  Line,
  LineChart,
  ResponsiveContainer,
  Tooltip,
  XAxis,
  YAxis,
} from "recharts";
import type { DisplayChannel as Channel, DisplaySeries as Series } from "@/api/types";
import { useT } from "@/i18n";
import { toneAt, toneHex } from "@/lib/tone";
import { numText } from "@/quantity/chart";

/** 標本が多すぎると描画が重いので間引く（表示だけ。値は変えない） */
function stride(n: number, max = 1500): number {
  return Math.max(1, Math.ceil(n / max));
}

/**
 * 時系列 1 組を、表示単位ごとに 1 枚ずつのグラフにする。表示値 = SI 値 × display_scale（単位の換算はしない）。
 * 横軸は経過時間 [s]。
 */
export function SeriesChart({ series, title }: { series: Series; title?: string }) {
  const tr = useT();
  const groups = new Map<string, Channel[]>();
  for (const c of series.channels)
    groups.set(c.display_unit, [...(groups.get(c.display_unit) ?? []), c]);
  const step = stride(series.t.length);
  return (
    <div className="space-y-4">
      {[...groups.entries()].map(([unit, chans]) => {
        const data: Record<string, number>[] = [];
        for (let i = 0; i < series.t.length; i += step) {
          const row: Record<string, number> = { t: series.t[i] ?? 0 };
          for (const c of chans) {
            const v = c.values[i];
            if (v !== undefined) row[c.key] = v * c.display_scale;
          }
          data.push(row);
        }
        return (
          <figure key={unit} className="m-0">
            <figcaption className="mb-1 text-ink-3 text-xs">
              {title ? `${title} · ` : ""}
              {chans.map((c) => c.label).join(tr.systems.seriesSep)}
              {tr.systems.seriesUnit(unit || "—")}
            </figcaption>
            <div className="h-56">
              <ResponsiveContainer width="100%" height="100%">
                <LineChart data={data} margin={{ top: 4, right: 12, bottom: 0, left: 0 }}>
                  <CartesianGrid stroke="var(--color-line)" vertical={false} />
                  <XAxis
                    dataKey="t"
                    type="number"
                    domain={["dataMin", "dataMax"]}
                    tickFormatter={(v: number) => numText(v)}
                    tick={{ fontSize: 11, fill: "var(--color-ink-3)" }}
                    label={{
                      value: tr.systems.elapsed,
                      position: "insideBottomRight",
                      fontSize: 10,
                      fill: "var(--color-ink-4)",
                    }}
                  />
                  <YAxis
                    tickFormatter={(v: number) => numText(v)}
                    tick={{ fontSize: 11, fill: "var(--color-ink-3)" }}
                    width={56}
                  />
                  <Tooltip
                    formatter={(v) => (typeof v === "number" ? `${numText(v)} ${unit}` : String(v))}
                    labelFormatter={(v) => `t = ${numText(Number(v))} s`}
                    contentStyle={{
                      fontSize: 12,
                      borderRadius: 9,
                      borderColor: "var(--color-line-strong)",
                    }}
                  />
                  {chans.length > 1 && <Legend wrapperStyle={{ fontSize: 11 }} />}
                  {chans.map((c, i) => (
                    <Line
                      key={c.key}
                      dataKey={c.key}
                      name={c.label}
                      dot={false}
                      isAnimationActive={false}
                      strokeWidth={1.5}
                      stroke={toneHex[toneAt(i)]}
                    />
                  ))}
                </LineChart>
              </ResponsiveContainer>
            </div>
          </figure>
        );
      })}
    </div>
  );
}
