import type { AnalysisLabel, ModalFemModeShape, ModalFemOutput, ModalFemShapes } from "@/api/types";
import { localized } from "@/i18n";

/** モードの選択肢 1 つ（振動数・種類・向きは API の値と表示名。ここでは作らない） */
export interface ModeRow {
  /** 1 から数えるモードの番号（`ModalFemMode.index`） */
  index: number;
  /** 固有振動数 [Hz]（SI。表示は `Formatted`） */
  frequency: number;
  /** 種類の表示名（API の `labels` の `/modes/<i>/class`。無ければ null） */
  classLabel: string | null;
  /** 最も効く軸の表示名（全体のモードだけ） */
  axisLabel: string | null;
  /** そのモードの形 */
  shape: ModalFemModeShape;
}

const labelOf = (labels: Readonly<Record<string, AnalysisLabel>>, pointer: string) => {
  const l = labels[pointer];
  return l ? localized(l.label, l.label_en) : null;
};

/**
 * 出力のモードと、形（`detail`）から選択肢を作る。番号で突き合わせ（並びの位置では比べない）、形のあるモードだけを出力の順に並べる。
 * 種類・向きの表示名は結果の `labels`（JSON Pointer → 表示名）から引く。
 */
export function modeRows(
  output: ModalFemOutput,
  labels: Readonly<Record<string, AnalysisLabel>>,
  shapes: ModalFemShapes,
): ModeRow[] {
  const byIndex = new Map(shapes.modes.map((m) => [m.index, m]));
  return output.modes.flatMap((m, i) => {
    const shape = byIndex.get(m.index);
    return shape
      ? [
          {
            index: m.index,
            frequency: m.frequency,
            classLabel: labelOf(labels, `/modes/${i}/class`),
            axisLabel: labelOf(labels, `/modes/${i}/dominant_axis`),
            shape,
          },
        ]
      : [];
  });
}
