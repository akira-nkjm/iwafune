import { Pause, Play } from "lucide-react";
import { useEffect, useId, useMemo, useState } from "react";
import type {
  DesignView,
  PartHeat,
  ThermalSteadyOutput,
  ThermalTransientOutput,
} from "@/api/types";
import { LazyLayout3D } from "@/features/layout3d/LazyLayout3D";
import { useT } from "@/i18n";
import { Empty } from "@/kit/feedback/Empty";
import { NativeSelect, type SelectOption } from "@/kit/forms/NativeSelect";
import { RangeInput } from "@/kit/forms/RangeInput";
import { heatHex } from "@/lib/tone";
import { Formatted } from "@/quantity/Formatted";
import { Button } from "@/ui/button";
import {
  type HeatScale,
  type HeatSource,
  heatRatio,
  heatScaleOf,
  nodeTemperatures,
  partHeatColors,
  transientTemperatures,
} from "./heat";
import { orbitView } from "./insetGeometry";
import { OrbitInset } from "./OrbitInset";
import { Temp } from "./RangeTip";

/** 再生で 1 回に進める時刻の数と間隔。全体を約 10 秒で再生する */
const FRAMES = 120;
const TICK_MS = 80;

/** 凡例の帯（CSS の線形グラデーション。色は `heatHex`） */
const gradientOf = () =>
  `linear-gradient(to right, ${Array.from({ length: 8 }, (_, i) => heatHex(i / 7)).join(", ")})`;

/** 選択肢の値 → 出どころ。値は選択肢の並びで決める（ケースの ID を文字列として分解しない） */
export function sourceOptions(
  steady: ThermalSteadyOutput | null,
  hasTransient: boolean,
  labels: { worst: string; steady: (name: string) => string; transient: string },
): { option: SelectOption; source: HeatSource }[] {
  const out: { option: SelectOption; source: HeatSource }[] = [
    { option: { value: "worst", label: labels.worst }, source: { kind: "worst" } },
  ];
  for (const [i, c] of (steady?.cases ?? []).entries())
    out.push({
      option: { value: `case-${i}`, label: labels.steady(c.label || c.id) },
      source: { kind: "case", id: c.id },
    });
  if (hasTransient)
    out.push({
      option: { value: "transient", label: labels.transient },
      source: { kind: "transient", index: 0 },
    });
  return out;
}

/**
 * 熱画像のタブ: 3D の部品を節点の温度で塗る。温度の出どころ（全ケースの最悪・定常の 1 ケース・非定常のある時刻）を選べて、
 * 色の範囲は出どころによらず全ケース・全時刻の最低〜最高で一定。部品と節点の対応は API（`part_heat`）で、ここは温度を引いて色に写すだけ。
 * 3D の部品の上のホバーは 3D の部品には無いので、横に節点の温度の一覧を置く。
 */
export function ThermalImage({
  design,
  steady,
  transient,
  heat,
}: {
  design: DesignView;
  steady: ThermalSteadyOutput | null;
  transient: ThermalTransientOutput | null;
  heat: readonly PartHeat[] | undefined;
}) {
  const t = useT();
  const I = t.systems.thermalImage;
  const series = useMemo(() => transientTemperatures(transient), [transient]);
  const scale = useMemo(() => heatScaleOf(steady, series), [steady, series]);
  const options = useMemo(
    () =>
      sourceOptions(steady, series !== null, {
        worst: I.worst,
        steady: I.steadyCase,
        transient: I.transient,
      }),
    [steady, series, I],
  );
  const [value, setValue] = useState("worst");
  const [index, setIndex] = useState(0);
  const first = {
    option: { value: "worst", label: I.worst },
    source: { kind: "worst" } as HeatSource,
  };
  const chosen = options.find((o) => o.option.value === value) ?? options[0] ?? first;
  const source = useMemo<HeatSource>(
    () => (chosen.source.kind === "transient" ? { kind: "transient", index } : chosen.source),
    [chosen, index],
  );
  const temps = useMemo(
    () => nodeTemperatures(source, steady, series, heat),
    [source, steady, series, heat],
  );
  const colorOf = useMemo(
    () => (scale ? partHeatColors(heat, temps, scale) : () => ""),
    [heat, temps, scale],
  );
  // 非定常のときだけ、同じ時刻の添字で軌道の図と太陽の矢印を出す（再生で添字が進むだけ。新しく取りに行かない）
  const view = orbitView(source, transient?.orbit_plane, index);
  if (!scale) return <Empty>{I.empty}</Empty>;
  return (
    // 3D を大きく見せるため、操作は 3D の見出しの帯、凡例は 3D の上、節点の一覧は 3D の右に置く（1 枚のパネル）
    <LazyLayout3D
      variant="heat"
      design={design}
      colorOf={colorOf}
      tools={
        <>
          <NativeSelect
            size="sm"
            aria-label={I.source}
            value={chosen.option.value}
            onChange={(e) => setValue(e.target.value)}
            options={options.map((o) => o.option)}
          />
          {series && chosen.source.kind === "transient" && (
            <TimeControl times={series.t} index={index} onIndex={setIndex} />
          )}
        </>
      }
      overlay={<Legend scale={scale} />}
      corner={view.plane && <OrbitInset plane={view.plane} index={index} />}
      sun={view.sun}
      aside={<NodeList temps={temps} heat={heat} scale={scale} />}
    />
  );
}

/** 非定常の時刻の選択: スライダと再生・停止、経過時間 */
function TimeControl({
  times,
  index,
  onIndex,
}: {
  times: readonly number[];
  index: number;
  onIndex: (i: number) => void;
}) {
  const I = useT().systems.thermalImage;
  const [playing, setPlaying] = useState(false);
  const id = useId();
  const last = times.length - 1;
  const at = Math.min(index, last);
  useEffect(() => {
    if (!playing || times.length <= 1) return;
    const step = Math.max(1, Math.ceil(times.length / FRAMES));
    const timer = setInterval(() => onIndex(at >= last ? 0 : Math.min(last, at + step)), TICK_MS);
    return () => clearInterval(timer);
  }, [playing, at, last, times.length, onIndex]);
  return (
    <div className="flex min-w-0 flex-1 flex-wrap items-center gap-3">
      <Button
        variant="outline"
        size="sm"
        aria-pressed={playing}
        onClick={() => setPlaying((p) => !p)}
      >
        {playing ? <Pause /> : <Play />}
        {playing ? I.pause : I.play}
      </Button>
      <RangeInput
        id={id}
        aria-label={I.time}
        min={0}
        max={last}
        step={1}
        value={at}
        onChange={onIndex}
        className="min-w-40 flex-1"
      />
      <label htmlFor={id} className="whitespace-nowrap text-ink-2 text-xs">
        {I.elapsed}{" "}
        <Formatted si={times[at] ?? 0} dimension="Time" fallback={String(times[at] ?? 0)} />
      </label>
    </div>
  );
}

/** 色の凡例: 範囲の両端の温度と帯。範囲は出どころを変えても動かない */
function Legend({ scale }: { scale: HeatScale }) {
  const I = useT().systems.thermalImage;
  return (
    <div className="flex items-center gap-2 text-ink-2 text-xs" role="img" aria-label={I.scale}>
      <Temp si={scale.min} />
      <span className="h-3 w-48 max-w-full rounded-sm" style={{ background: gradientOf() }} />
      <Temp si={scale.max} />
    </div>
  );
}

/** 節点の温度の一覧（色の見本・節点・その節点に入っている部品・温度） */
function NodeList({
  temps,
  heat,
  scale,
}: {
  temps: ReadonlyMap<string, number>;
  heat: readonly PartHeat[] | undefined;
  scale: HeatScale;
}) {
  const I = useT().systems.thermalImage;
  return (
    <>
      <h3 className="mb-2 font-medium text-ink text-sm">{I.nodes}</h3>
      <ul className="space-y-1 text-xs">
        {[...temps].map(([node, temp]) => (
          <li key={node} className="flex items-center gap-2">
            <span
              aria-hidden
              className="size-3 shrink-0 rounded-sm"
              style={{ background: heatHex(heatRatio(temp, scale)) }}
            />
            <span className="min-w-0 flex-1 truncate font-mono text-ink-2">
              {node}
              <span className="text-ink-4">
                {" "}
                {(heat ?? [])
                  .filter((h) => h.node === node)
                  .map((h) => h.part)
                  .join(", ")}
              </span>
            </span>
            <Temp si={temp} />
          </li>
        ))}
      </ul>
    </>
  );
}
