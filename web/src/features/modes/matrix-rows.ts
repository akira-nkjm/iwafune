import type { DesignView, ModeLoadOutput, ModeRow, ModeTable } from "@/api/types";
import { kindLabel, nodeLabel, partById } from "@/lib/design";
import { type CellText, cellOf, type ModeDraft } from "./draft";
import { type CellWords, powerLevels, savedCellText } from "./matrix";

/** 行列の 1 セルに出すもの（`ModeCell` に渡す） */
export interface MatrixCell {
  mode: string;
  /** 下書きがあれば下書きのセル、無ければ保存済みのセル（未指定は null） */
  cell: CellText | null;
  /** 保存済みのセルの文言（`savedCellText`）。そのモードに書かれていなければ undefined */
  display: string | undefined;
  /** 下書きで変えたセルか */
  changed: boolean;
  /** そのモードでのその部品の平均消費電力 [W]（power.mode_load の出力。無ければ null） */
  power: number | null;
  /** 塗りの段階（今の状態が電力レベルの何段目か。オフ・未指定は -1） */
  shade: number;
}

/** 行列の 1 行（部品）に出すもの */
export interface MatrixRow {
  part: string;
  /** 員数（2 以上なら `×n` を添える） */
  count: number;
  /** 種別の表示名（種別が無ければ null） */
  kind: string | null;
  /** 電力レベルの列（状態ごとの電力を小さい順に。API の整形）と、その見出し（状態の名前付き） */
  levels: string;
  levelsTitle: string;
  /** 部品の状態の語彙（セルの切り替えとポップオーバー） */
  states: string[];
  /** 行列の中の行の番号（矢印キーの移動） */
  index: number;
  cells: MatrixCell[];
}

/** サブシステム（部品の Group）ごとのまとまり */
export interface MatrixGroup {
  group: string | null;
  /** まとまりの見出し（ノードの表示名） */
  label: string;
  rows: MatrixRow[];
}

/** 部品の行の中のセル（下書き・保存済みの文言・下書きを当てた電力） */
function cellsOf(
  table: ModeTable,
  draft: ModeDraft,
  load: ModeLoadOutput | null,
  row: ModeRow,
  levels: ReturnType<typeof powerLevels>,
  words: CellWords,
): MatrixCell[] {
  return table.modes.map((m) => {
    const cell = cellOf(table, draft, m.id, row.part);
    const power =
      load?.modes.find((mode) => mode.id === m.id)?.parts.find((p) => p.part === row.part)?.mean ??
      null;
    return {
      mode: m.id,
      cell,
      display: savedCellText(row, m.id, words),
      changed: draft.get(m.id)?.has(row.part) ?? false,
      power,
      shade: levels.findIndex(([state]) => state === cell?.state),
    };
  });
}

/**
 * モード表（API の `ModeTable`）・下書き・下書きを当てた power.mode_load の出力 → 行列に出す行。
 * 電力を使う部品（状態のどれかの電力が 0 より大きい）だけを、サブシステムの ID・部品の ID の順に、
 * サブシステムごとにまとめて並べる。値の計算はせず、API の値を並べるだけ。
 */
export function matrixGroups(
  design: DesignView,
  draft: ModeDraft,
  load: ModeLoadOutput | null,
  words: CellWords,
): MatrixGroup[] {
  const table = design.modes;
  const parts = partById(design);
  const groupOf = (row: ModeRow) => parts.get(row.part)?.subsystem ?? null;
  const rows = table.rows
    .filter((r) => powerLevels(parts.get(r.part)).some(([, q]) => q.si > 0))
    .sort(
      (a, b) => (groupOf(a) ?? "").localeCompare(groupOf(b) ?? "") || a.part.localeCompare(b.part),
    );
  const groups: MatrixGroup[] = [];
  rows.forEach((row, index) => {
    const group = groupOf(row);
    const part = parts.get(row.part);
    const levels = powerLevels(part);
    let last = groups.at(-1);
    if (!last || last.group !== group) {
      last = { group, label: nodeLabel(design, group), rows: [] };
      groups.push(last);
    }
    last.rows.push({
      part: row.part,
      count: part?.count ?? 1,
      kind: row.kind && kindLabel(design.defs, row.kind),
      levels: levels.map(([, q]) => q.display).join(" / "),
      levelsTitle: levels.map(([state, q]) => `${state}: ${q.display}`).join(" / "),
      states: row.states,
      index,
      cells: cellsOf(table, draft, load, row, levels, words),
    });
  });
  return groups;
}
