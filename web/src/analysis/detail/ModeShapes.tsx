import { useId, useMemo, useState } from "react";
import { analysisOutput } from "@/api/analysis-output";
import type { AnalysisRunResult, ModalFemShapes } from "@/api/types";
import { useT } from "@/i18n";
import { RangeInput } from "@/kit/forms/RangeInput";
import { FieldLegend, SurfaceField } from "@/kit/scene3d/SurfaceField";
import type { V3 } from "@/kit/scene3d/types";
import { Switch } from "@/kit/selection/Switch";
import { cn } from "@/lib/cn";
import { Formatted } from "@/quantity/Formatted";
import { type ModeRow, modeRows } from "./mode-shapes";

/** 表示の倍率の既定と上限（最大の変位を、機体の大きさの何 % で描くか。表示だけの量） */
const DEFAULT_AMPLITUDE_PERCENT = 10;
const MAX_AMPLITUDE_PERCENT = 30;

/** 動きを減らす設定（OS）のとき、アニメーションは止めて始める（押して動かすのは自由） */
function prefersReducedMotion(): boolean {
  return (
    typeof window !== "undefined" &&
    window.matchMedia?.("(prefers-reduced-motion: reduce)").matches === true
  );
}

/**
 * 構造の有限要素の固有値解析（`structure.modal_fem`）のモードの形: 選んだモードの変形したメッシュを、変位の大きさの色で 3D に出す。
 * 形・規格化・メッシュは API の詳細（`ModalFemShapes`）、モードの振動数・種類・向きは出力と `labels`。ここは選んで描くだけ。
 */
export function ModeShapes({
  result,
  shapes,
}: {
  result: AnalysisRunResult;
  shapes: ModalFemShapes;
}) {
  const output = analysisOutput(result, "structure.modal_fem");
  const rows = useMemo(
    () => (output ? modeRows(output, result.labels, shapes) : []),
    [output, result.labels, shapes],
  );
  const [chosen, setChosen] = useState<number | null>(null);
  const [percent, setPercent] = useState(DEFAULT_AMPLITUDE_PERCENT);
  const amplitudeId = useId();
  const [animate, setAnimate] = useState(() => !prefersReducedMotion());
  const t = useT().systems.modeShapes;
  const row = rows.find((r) => r.index === chosen) ?? rows[0];
  if (!row) return <p className="text-ink-3 text-sm">{t.empty}</p>;
  return (
    <section aria-label={t.title} className="space-y-2">
      <h3 className="font-medium text-ink-2 text-xs">{t.title}</h3>
      <div className="grid gap-3 lg:grid-cols-[minmax(0,17rem)_minmax(0,1fr)]">
        <ModeList rows={rows} selected={row.index} onSelect={setChosen} />
        <div className="min-w-0 space-y-2">
          <div className="relative h-[420px] overflow-hidden rounded-control border border-line">
            <SurfaceField
              mesh={shapes.mesh}
              displacement={row.shape.displacement as V3[]}
              values={row.shape.magnitude}
              amplitude={percent / 100}
              animate={animate}
              className="relative h-full"
            />
          </div>
          <div className="flex flex-wrap items-center gap-x-4 gap-y-2">
            <label
              htmlFor={amplitudeId}
              className="flex min-w-0 flex-wrap items-center gap-2 text-ink-2 text-xs"
            >
              {t.amplitude}
              <RangeInput
                id={amplitudeId}
                aria-label={t.amplitude}
                min={0}
                max={MAX_AMPLITUDE_PERCENT}
                step={1}
                value={percent}
                onChange={setPercent}
                className="w-32 max-w-full"
              />
              <span className="num w-10 text-right">{`${percent} %`}</span>
            </label>
            <Switch value={animate} onChange={setAnimate}>
              {t.animate}
            </Switch>
            <FieldLegend low={t.legendLow} high={t.legendHigh} label={t.legend} />
          </div>
          <p className="text-ink-3 text-xs">{t.note}</p>
        </div>
      </div>
    </section>
  );
}

/** モードの選択肢（振動数・種類・向きを並べる。押したモードの形に切り替わる） */
export function ModeList({
  rows,
  selected,
  onSelect,
}: {
  rows: readonly ModeRow[];
  selected: number;
  onSelect: (index: number) => void;
}) {
  const t = useT().systems.modeShapes;
  return (
    <div role="radiogroup" aria-label={t.modes} className="space-y-1">
      {rows.map((r) => {
        const on = r.index === selected;
        return (
          // biome-ignore lint/a11y/useSemanticElements: 振動数に書式（Formatted）を入れるので、ネイティブの radio は使えない
          <button
            key={r.index}
            type="button"
            role="radio"
            aria-checked={on}
            onClick={() => onSelect(r.index)}
            className={cn(
              "flex w-full flex-col gap-0.5 rounded-control border px-3 py-1.5 text-left text-xs outline-none focus-visible:ring-2 focus-visible:ring-accent/40",
              on
                ? "border-accent bg-accent-tint text-ink"
                : "border-line text-ink-2 hover:bg-panel-2",
            )}
          >
            <span className="flex items-baseline gap-2">
              <span className="text-ink-3">{t.mode(r.index)}</span>
              <Formatted si={r.frequency} dimension="Frequency" fallback={String(r.frequency)} />
            </span>
            <span className="text-ink-3">
              {[r.classLabel, r.axisLabel].filter((x) => x !== null).join(" · ")}
            </span>
          </button>
        );
      })}
    </div>
  );
}
