import type { CommandText, ThermalCellText } from "@/api/types";

/**
 * 熱の結合・表面の下書き: 変えたセル。同じ対象への編集は 1 つにまとめる（後の編集が前の編集を置き換える）。
 * セルは `fune set|unset analysis.thermal.…` の宛先と値の組で、パスの組み立てと値の型は service が決める。
 */
export type ThermalDraft = ReadonlyMap<string, ThermalCellText>;

/** 同じ対象かの見分け（セルの宛先。文字列のつなぎ方に意味は持たせない） */
export function cellKey(cell: ThermalCellText): string {
  switch (cell.cell) {
    case "coupling":
      return JSON.stringify([cell.cell, cell.kind, cell.a, cell.b]);
    case "coating":
      // 1 セル 1 欄（`coatingCell`）なので、書いた欄が宛先の一部
      return JSON.stringify([
        cell.cell,
        cell.id,
        cell.label != null ? "label" : cell.absorptivity != null ? "absorptivity" : "emissivity",
      ]);
    case "coating_remove":
      return JSON.stringify([cell.cell, cell.id]);
    case "surface":
      return JSON.stringify([cell.cell, cell.part, cell.coating]);
    case "surface_remove":
      return JSON.stringify([cell.cell, cell.part]);
  }
}

/** セルを加えた下書き（同じ対象のセルは置き換える。順序は最初に加えた順） */
export function withCell(draft: ThermalDraft, cell: ThermalCellText): ThermalDraft {
  const next = new Map(draft);
  next.set(cellKey(cell), cell);
  return next;
}

/** セルを外した下書き */
export function withoutCell(draft: ThermalDraft, key: string): ThermalDraft {
  const next = new Map(draft);
  next.delete(key);
  return next;
}

/** 下書き → `POST /ops/from_command` の引数（Op にするのは service）。下書きが無ければ null */
export function draftCommand(draft: ThermalDraft): CommandText | null {
  return draft.size > 0 ? { command: "thermal_set", cells: [...draft.values()] } : null;
}

/** 表面材の 1 つの欄の編集（欄ごとに 1 セル） */
export function coatingCell(
  id: string,
  field: "label" | "absorptivity" | "emissivity",
  text: string,
): ThermalCellText {
  return { cell: "coating", id, label: null, absorptivity: null, emissivity: null, [field]: text };
}
