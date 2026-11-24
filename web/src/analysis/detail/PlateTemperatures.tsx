import { useMemo, useState } from "react";
import type { ThermalPlateTemperatures } from "@/api/types";
import { useT } from "@/i18n";
import { NativeSelect } from "@/kit/forms/NativeSelect";
import { FieldLegend } from "@/kit/scene3d/FieldLegend";
import { SurfaceField } from "@/kit/scene3d/SurfaceField";
import { cn } from "@/lib/cn";
import { type HeatScale, heatColor } from "@/lib/heat-scale";
import { Temp } from "@/quantity/Temp";
import {
  cellRatios,
  detailScale,
  type FieldChoice,
  fieldChoices,
  type PlateRows,
  plateRows,
} from "./plate-temperatures";

/**
 * 升に分けた板の温度の分布（`thermal.steady` と `thermal.transient` の詳細）: 板を機体の中の実際の位置で、升ごとの温度の色で 3D に出す。
 * 色は熱画像と同じ配色（`heatHex`）と同じ意味の凡例（範囲は表示できる全部の場の最低〜最高）。場（定常はケース、非定常は最低・最高・終わり）を
 * 選べて、升は 3D と一覧のどちらに重ねても升の名前と温度を読める。メッシュ・升の温度・並びは API の詳細（`ThermalPlateTemperatures`）。
 */
export function PlateTemperatures({ detail }: { detail: ThermalPlateTemperatures }) {
  const t = useT().systems.plateTemperatures;
  const choices = useMemo(
    () => fieldChoices(detail.fields, { steady: t.steady, quantity: t.quantity }),
    [detail.fields, t],
  );
  const scale = useMemo(() => detailScale(detail), [detail]);
  const [value, setValue] = useState("0");
  const [hover, setHover] = useState<number | null>(null);
  const choice = choices.find((c) => c.value === value) ?? choices[0];
  const ratios = useMemo(
    () => (choice && scale ? cellRatios(choice.field, scale) : []),
    [choice, scale],
  );
  const rows = useMemo(() => (choice ? plateRows(detail, choice.field) : []), [detail, choice]);
  if (!choice || !scale) return null;
  const hovered = hover === null ? undefined : detail.cells[hover];
  return (
    <section aria-label={t.title} className="@container space-y-2">
      <h3 className="font-medium text-ink-2 text-xs">{t.title}</h3>
      {/* 幅は枠（解析のカードは半分の幅、行の中は全幅）で決める。狭いときは一覧を 3D の下に積む */}
      <div className="grid gap-3 @3xl:grid-cols-[minmax(0,1fr)_minmax(0,17rem)]">
        <div className="min-w-0 space-y-2">
          <div className="relative h-[420px] overflow-hidden rounded-control border border-line">
            <SurfaceField
              mesh={detail.mesh}
              values={ratios}
              at="cell"
              highlight={hover}
              onHover={setHover}
              className="relative h-full"
            />
          </div>
          <div className="flex flex-wrap items-center gap-x-4 gap-y-2">
            <FieldSelect choices={choices} value={choice.value} onChange={setValue} />
            <FieldLegend
              low={<Temp si={scale.min} />}
              high={<Temp si={scale.max} />}
              label={t.legend}
            />
          </div>
          {/* 読み出しの行は常に 1 行分の高さを取る（ホバーで下の文が動かないように） */}
          <p className="min-h-5 text-ink-2 text-xs" aria-live="polite">
            {hovered && hover !== null ? (
              <>
                <span className="font-mono">{hovered.node}</span>{" "}
                <Temp si={choice.field.temperature[hover] ?? Number.NaN} />
              </>
            ) : (
              <span className="text-ink-3">{t.hint}</span>
            )}
          </p>
          <p className="text-ink-3 text-xs">{t.note}</p>
        </div>
        <CellList rows={rows} scale={scale} hover={hover} onHover={setHover} />
      </div>
    </section>
  );
}

/** 場の選択（選択肢が 1 つだけなら出さない） */
export function FieldSelect({
  choices,
  value,
  onChange,
}: {
  choices: readonly FieldChoice[];
  value: string;
  onChange: (value: string) => void;
}) {
  const t = useT().systems.plateTemperatures;
  if (choices.length <= 1) return null;
  return (
    <NativeSelect
      size="sm"
      aria-label={t.source}
      value={value}
      onChange={(e) => onChange(e.target.value)}
      options={choices.map((c) => ({ value: c.value, label: c.label }))}
    />
  );
}

/** 升の一覧（板ごと。色の見本・升の名前・温度）。升に重ねると 3D の升が強調される */
export function CellList({
  rows,
  scale,
  hover,
  onHover,
}: {
  rows: readonly PlateRows[];
  scale: HeatScale;
  hover: number | null;
  onHover: (index: number | null) => void;
}) {
  const t = useT().systems.plateTemperatures;
  return (
    <section
      aria-label={t.cells}
      className="max-h-60 min-w-0 space-y-3 overflow-y-auto @3xl:max-h-[420px]"
    >
      {rows.map((plate) => (
        <section key={plate.part} aria-label={plate.part}>
          <h4 className="mb-1 font-medium text-ink text-xs">
            {t.plate(plate.part, plate.size[0], plate.size[1])}
          </h4>
          <ul className="space-y-0.5 text-xs">
            {plate.cells.map((c) => (
              <li
                key={c.node}
                onMouseEnter={() => onHover(c.index)}
                onMouseLeave={() => onHover(null)}
                className={cn(
                  "flex items-center gap-2 rounded-sm px-1",
                  hover === c.index && "bg-accent-tint",
                )}
              >
                <span
                  aria-hidden
                  className="size-3 shrink-0 rounded-sm"
                  style={{ background: heatColor(c.temperature, scale) }}
                />
                <span className="min-w-0 flex-1 truncate font-mono text-ink-2" title={c.node}>
                  {c.node}
                </span>
                <span className="w-16 shrink-0 text-right">
                  <Temp si={c.temperature} />
                </span>
              </li>
            ))}
          </ul>
        </section>
      ))}
    </section>
  );
}
