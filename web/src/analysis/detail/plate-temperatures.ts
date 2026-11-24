import type {
  ThermalPlateField,
  ThermalPlateQuantity,
  ThermalPlateTemperatures,
} from "@/api/types";
import { type HeatScale, heatRatio, heatScaleOfValues } from "@/lib/heat-scale";

/** 場の選択肢 1 つ。値は並びの添字（ケースの ID を文字列として分解しない） */
export interface FieldChoice {
  value: string;
  label: string;
  field: ThermalPlateField;
}

/**
 * 場の選択肢。定常はケースの表示名（`steady(name)`。ケースの `label`、無ければ ID）、非定常は量の名前（最低・最高・終わり）。
 * 並びは API の `fields` のまま。
 */
export function fieldChoices(
  fields: readonly ThermalPlateField[],
  labels: { steady: (name: string) => string; quantity: Record<ThermalPlateQuantity, string> },
): FieldChoice[] {
  return fields.map((field, i) => ({
    value: String(i),
    label:
      field.quantity === "steady"
        ? labels.steady(field.label || field.case || labels.quantity.steady)
        : labels.quantity[field.quantity],
    field,
  }));
}

/**
 * 色の範囲: 表示できるすべての場（全ケース・最低/最高/終わり）の升の温度の最低〜最高。場を切り替えても色の意味は同じ
 * （熱画像がケースや時刻を切り替えても範囲が動かないのと同じ）。値が無ければ null。
 */
export function detailScale(detail: ThermalPlateTemperatures): HeatScale | null {
  return heatScaleOfValues(detail.fields.flatMap((f) => f.temperature));
}

/** 場の升ごとの色の位置（0 〜 1。`SurfaceField` の `values`。四角形と同じ並び） */
export function cellRatios(field: ThermalPlateField, scale: HeatScale): number[] {
  return field.temperature.map((t) => heatRatio(t, scale));
}

/** 一覧の 1 板分 */
export interface PlateRows {
  part: string;
  /** 升の数 (u, v)（API の `ThermalPlateCell` の番号の最大 + 1） */
  size: [number, number];
  cells: { index: number; node: string; temperature: number }[];
}

/** 板ごとの升の一覧（板は `cells` に出てきた順、升は API の並び）。`index` は四角形の通し番号（`SurfaceField` の `highlight` と同じ） */
export function plateRows(detail: ThermalPlateTemperatures, field: ThermalPlateField): PlateRows[] {
  const out = new Map<string, PlateRows>();
  detail.cells.forEach((c, index) => {
    const rows = out.get(c.cell.part) ?? { part: c.cell.part, size: [0, 0], cells: [] };
    rows.size = [Math.max(rows.size[0], c.cell.i + 1), Math.max(rows.size[1], c.cell.j + 1)];
    rows.cells.push({ index, node: c.node, temperature: field.temperature[index] ?? Number.NaN });
    out.set(c.cell.part, rows);
  });
  return [...out.values()];
}
