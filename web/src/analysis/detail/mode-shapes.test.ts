import { describe, expect, it } from "vitest";
import type { ModalFemOutput, ModalFemShapes } from "@/api/types";
import { modeRows } from "./mode-shapes";

const shape = (index: number) => ({
  index,
  displacement: [[0, 0, 1]] as [number, number, number][],
  magnitude: [1],
});
const mode = (index: number, frequency: number) =>
  ({ index, frequency }) as unknown as ModalFemOutput["modes"][number];

const output = { modes: [mode(1, 70), mode(2, 82), mode(3, 121)] } as unknown as ModalFemOutput;
const labels = {
  "/modes/0/class": { label: "局所", label_en: "Local" },
  "/modes/1/class": { label: "全体・横", label_en: "Global, lateral" },
  "/modes/1/dominant_axis": { label: "X 軸", label_en: "X axis" },
};

describe("モードの選択肢", () => {
  it("形のあるモードだけを、出力の順に、API の表示名つきで並べる", () => {
    const shapes = { modes: [shape(2), shape(1)] } as unknown as ModalFemShapes;
    const rows = modeRows(output, labels, shapes);
    expect(rows.map((r) => r.index)).toEqual([1, 2]);
    expect(rows[1]).toMatchObject({
      frequency: 82,
      classLabel: "全体・横",
      axisLabel: "X 軸",
    });
    // 向きの表示名が無い（局所のモード）は null
    expect(rows[0]).toMatchObject({ classLabel: "局所", axisLabel: null });
  });

  it("番号で突き合わせる（形の並びの位置には頼らない）", () => {
    const shapes = { modes: [shape(3)] } as unknown as ModalFemShapes;
    const rows = modeRows(output, labels, shapes);
    expect(rows.map((r) => r.index)).toEqual([3]);
    expect(rows[0]?.classLabel).toBeNull();
  });
});
