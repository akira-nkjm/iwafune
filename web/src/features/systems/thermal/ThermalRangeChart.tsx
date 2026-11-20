import { type ComponentProps, useId, useState } from "react";
import type { ThermalRangeKind, Verdict } from "@/api/types";
import { useT } from "@/i18n";
import { Panel } from "@/kit/panel/Panel";
import { PanelHeader } from "@/kit/panel/PanelHeader";
import { cn } from "@/lib/cn";
import { RangeTip, Temp } from "./RangeTip";
import {
  axisPosition,
  axisTicks,
  RANGE_KINDS,
  type RangeChartModel,
  type RangeRow,
  type RangeSpan,
} from "./rangeChart";

/** 帯の塗り（外側の保管ほど淡く、内側の性能ほど濃い） */
const BAND_FILL = {
  storage: "bg-sunken",
  operating: "bg-pos/25",
  performance: "bg-pos/50",
} as const satisfies Record<ThermalRangeKind, string>;

/** 帯の行の中での縦の位置（外側の保管ほど背が高く、内側の性能ほど低い入れ子） */
const BAND_INSET = {
  storage: "inset-y-0.5",
  operating: "inset-y-2",
  performance: "inset-y-3",
} as const satisfies Record<ThermalRangeKind, string>;

const MARK_FILL = {
  ok: "bg-accent text-inverse",
  warning: "bg-warn text-ink",
  violation: "bg-neg text-inverse",
} as const satisfies Record<Verdict, string>;

/** 判定の印（注意・違反は色に加えて記号でも示す） */
const MARK = { ok: "", warning: "!", violation: "×" } as const satisfies Record<Verdict, string>;

const RING = {
  ok: "",
  warning: "ring-2 ring-warn",
  violation: "ring-2 ring-neg",
} as const satisfies Record<Verdict, string>;

/** 吹き出しを置く側（行の左半分なら左端そろえ、右半分なら右端そろえ） */
const tipSide = (pos: number) => (pos < 0.5 ? "" : "-translate-x-full");

const pct = (x: number) => `${x * 100}%`;

type Active = { node: string; key: string } | null;

/**
 * 節点ごとの温度と範囲の図。横軸は共通の温度、行は節点。背景の帯が温度範囲（保管 ⊃ 動作 ⊃ 性能）、
 * 丸が定常のケース（選んだケースは強調、ほかは薄く）、棒が非定常の最低〜最高。判定と余裕は API の値で、ここでは計算しない。
 */
export function ThermalRangeChart({
  model,
  kase,
}: {
  model: RangeChartModel;
  /** ページの上で選んだケース（強調する）。図のケースに無ければ強調しない */
  kase: string | null;
}) {
  const t = useT();
  const R = t.systems.rangeChart;
  const [active, setActive] = useState<Active>(null);
  const emphasis = kase !== null && model.caseIds.includes(kase) ? kase : null;
  const ticks = axisTicks(model.lo, model.hi);
  return (
    <Panel>
      <PanelHeader title={R.title} caption={R.caption} />
      <div>
        <div className="pr-8">
          <div className="flex items-end">
            <div className="w-28 shrink-0" />
            <div role="presentation" className="relative h-6 min-w-0 flex-1">
              {ticks.map((v) => (
                <span
                  key={v}
                  className="absolute bottom-0 -translate-x-1/2 text-[11px] text-ink-3"
                  style={{ left: pct(axisPosition(v, model.lo, model.hi)) }}
                >
                  <Temp si={v} />
                </span>
              ))}
            </div>
          </div>
          {model.rows.map((row) => (
            <Row
              key={row.node}
              row={row}
              model={model}
              ticks={ticks}
              emphasis={emphasis}
              active={active}
              onActive={setActive}
            />
          ))}
        </div>
      </div>
      <ul className="mt-2 flex flex-wrap gap-x-4 gap-y-1 text-ink-3 text-xs">
        {RANGE_KINDS.map((k) => (
          <li key={k} className="flex items-center gap-1.5">
            <span className={cn("inline-block h-3 w-6 rounded-sm", BAND_FILL[k])} />
            {R.kinds[k]}
          </li>
        ))}
        <li>{R.legendMarker}</li>
        <li>{R.legendSpan}</li>
      </ul>
    </Panel>
  );
}

/** 1 節点の行 */
function Row({
  row,
  model,
  ticks,
  emphasis,
  active,
  onActive,
}: {
  row: RangeRow;
  model: RangeChartModel;
  ticks: readonly number[];
  emphasis: string | null;
  active: Active;
  onActive: (a: Active) => void;
}) {
  const t = useT();
  const R = t.systems.rangeChart;
  const uid = useId();
  const at = (v: number) => axisPosition(v, model.lo, model.hi);
  const tip = (key: string) => `${uid}-${key}`;
  const on = (key: string) => ({
    onMouseEnter: () => onActive({ node: row.node, key }),
    onMouseLeave: () => onActive(null),
    onFocus: () => onActive({ node: row.node, key }),
    onBlur: () => onActive(null),
    "aria-describedby": active?.node === row.node && active.key === key ? tip(key) : undefined,
  });
  const shown = active?.node === row.node ? active.key : null;
  return (
    <div className="flex items-center border-line-soft border-t">
      <div className="w-28 shrink-0 truncate pr-2 font-mono text-ink-2 text-xs" title={row.node}>
        {row.node}
      </div>
      <div className="relative h-9 min-w-0 flex-1">
        {ticks.map((v) => (
          <span
            key={v}
            aria-hidden
            className="absolute inset-y-0 border-line-soft border-l"
            style={{ left: pct(at(v)) }}
          />
        ))}
        {row.bands.map((b) => (
          <span
            key={b.kind}
            aria-hidden
            className={cn("absolute rounded-sm", BAND_FILL[b.kind], BAND_INSET[b.kind])}
            style={{ left: pct(at(b.min)), width: pct(at(b.max) - at(b.min)) }}
          />
        ))}
        {row.span && (
          <SpanBar
            span={row.span}
            left={at(row.span.min)}
            right={at(row.span.max)}
            label={`${row.node} ${R.transient}`}
            handlers={on("transient")}
          />
        )}
        {row.markers.map((m) => (
          <button
            key={m.caseId}
            type="button"
            aria-label={`${row.node} ${m.label}: ${t.verdict[m.verdict]}`}
            {...on(m.caseId)}
            className={cn(
              "-translate-x-1/2 -translate-y-1/2 absolute top-1/2 inline-flex size-4 items-center justify-center rounded-full font-bold text-[10px] leading-none outline-none focus-visible:ring-2 focus-visible:ring-ink",
              MARK_FILL[m.verdict],
              m.caseId === emphasis ? "z-10 ring-2 ring-ink" : emphasis && "opacity-40",
            )}
            style={{ left: pct(at(m.temperature)) }}
          >
            {MARK[m.verdict]}
          </button>
        ))}
        {row.markers.map(
          (m) =>
            shown === m.caseId && (
              <RangeTip
                key={m.caseId}
                id={tip(m.caseId)}
                title={m.label}
                temperature={<Temp si={m.temperature} />}
                ranges={m.ranges}
                verdict={m.verdict}
                className={tipSide(at(m.temperature))}
              />
            ),
        )}
        {row.span && shown === "transient" && (
          <RangeTip
            id={tip("transient")}
            title={R.transient}
            temperature={
              <>
                <Temp si={row.span.min} />
                {t.systems.rangeSep}
                <Temp si={row.span.max} />
              </>
            }
            ranges={row.span.ranges}
            verdict={row.span.verdict}
            className={tipSide(at(row.span.min))}
          />
        )}
      </div>
    </div>
  );
}

/** 非定常の最低〜最高の棒。細くなりすぎないよう最小の幅を持つ。注意・違反は枠の色と記号 */
function SpanBar({
  span,
  left,
  right,
  label,
  handlers,
}: {
  span: RangeSpan;
  left: number;
  right: number;
  label: string;
  handlers: ComponentProps<"button">;
}) {
  const t = useT();
  return (
    <button
      type="button"
      aria-label={`${label}: ${t.verdict[span.verdict]}`}
      {...handlers}
      className={cn(
        "-translate-y-1/2 absolute top-1/2 inline-flex h-3 min-w-1.5 items-center justify-center rounded-full bg-ink-3/60 font-bold text-[10px] text-ink leading-none outline-none focus-visible:ring-2 focus-visible:ring-ink",
        RING[span.verdict],
      )}
      style={{ left: pct(left), width: pct(right - left) }}
    >
      {MARK[span.verdict]}
    </button>
  );
}
