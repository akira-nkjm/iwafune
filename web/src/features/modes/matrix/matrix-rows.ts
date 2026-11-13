import type {
  DesignView,
  ModeRow,
  ModeTable,
  PartRow,
  QuantityView,
  StateAssign,
  System,
  SystemModesView,
} from "@/api/types";
import { kindLabel, nodeLabel, partById } from "@/lib/design";
import { type CellText, cellOf, draftKey, type ModeDraft } from "../draft/draft";
import { type CellWords, powerLevels, savedCellText } from "./matrix";
import type { CellAt } from "./matrix-focus";

/** 行列の 1 セルに出すもの（`ModeCell` はこれ 1 つで描く） */
export interface MatrixCell {
  eclipse?: boolean;
  override?: boolean;
  savedDuty?: StateAssign["duty"];
  mode: string;
  part: string;
  /** 部品の状態の語彙（セルの切り替えとポップオーバー） */
  states: string[];
  /** 下書きがあれば下書きのセル、無ければ保存済みのセル（未指定は null） */
  value: CellText | null;
  /** 保存済みのセルの文言（`savedCellText`）。そのモードに書かれていなければ undefined */
  display: string | undefined;
  /** 下書きで変えたセルか */
  changed: boolean;
  /** その系の部品別の値（API の量。無ければ null） */
  quantity: QuantityView | null;
  /** 塗りの段階（今の状態が電力レベルの何段目か。オフ・未指定は -1） */
  shade: number;
  /** 行列の中の位置（矢印キーの移動。行は部品の通し番号、列はモードの順） */
  at: CellAt;
}

/** セルを書き換えたとき: どのセルか（モードと部品）と、新しいセル（null は未指定に戻す） */
export type OnCell = (cell: Pick<MatrixCell, "mode" | "part">, value: CellText | null) => void;

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
  cells: MatrixCell[];
}

/** サブシステム（部品の Group）ごとのまとまり */
export interface MatrixGroup {
  group: PartRow["system"];
  /** まとまりの見出し（ノードの表示名） */
  label: string;
  rows: MatrixRow[];
}

/** 部品 1 つのセルを作るのに要るもの（モード表の行・行列の中の行の番号・電力レベル・下書きを当てた電力） */
interface RowSource {
  row: ModeRow;
  index: number;
  levels: ReturnType<typeof powerLevels>;
  /** モード → この系の部品別の値（無ければ null） */
  quantity: (mode: string) => QuantityView | null;
}

/** 部品の行の中のセル（下書き・保存済みの文言・下書きを当てた電力） */
function cellsOf(
  table: ModeTable,
  draft: ModeDraft,
  { row, index, levels, quantity }: RowSource,
  words: CellWords,
  eclipse: boolean,
): MatrixCell[] {
  return table.modes.map((m, column) => {
    const override = eclipse && cellOf(table, draft, m.id, row.part, true) !== null;
    const sourceEclipse = eclipse && override;
    const value = cellOf(table, draft, m.id, row.part, sourceEclipse);
    return {
      eclipse,
      override,
      savedDuty: sourceEclipse ? row.eclipse_cells[m.id]?.duty : undefined,
      mode: m.id,
      part: row.part,
      states: row.states,
      value,
      display:
        sourceEclipse || draft.get(m.id)?.has(row.part)
          ? undefined
          : savedCellText(row, m.id, words),
      changed:
        (draft.get(draftKey(m.id, eclipse))?.has(row.part) ?? false) ||
        (eclipse && !override && (draft.get(m.id)?.has(row.part) ?? false)),
      quantity: quantity(m.id),
      shade: levels.findIndex(([state]) => state === value?.state),
      at: { row: index, column },
    };
  });
}

/**
 * モード表（API の `ModeTable`）・下書き・下書きを当てた系別 DTO → 行列に出す行。
 * 定義では全ての部品を、系別表示では API が値を返す部品を、サブシステムの ID・部品の ID の順に、
 * サブシステムごとにまとめて並べる。値の計算はせず、API の値を並べるだけ。
 */
export function matrixGroups(
  design: DesignView,
  draft: ModeDraft,
  view: SystemModesView | null,
  words: CellWords,
  eclipse = false,
  system?: System,
): MatrixGroup[] {
  const table = design.modes;
  const parts = partById(design);
  const groupOf = (row: ModeRow) => parts.get(row.part)?.system ?? null;
  const rows = table.rows
    .filter(
      (r) =>
        !system || view?.modes.some((m) => r.part in m.sunlit.parts || r.part in m.eclipse.parts),
    )
    .sort(
      (a, b) => (groupOf(a) ?? "").localeCompare(groupOf(b) ?? "") || a.part.localeCompare(b.part),
    );
  const groups: MatrixGroup[] = [];
  rows.forEach((row, index) => {
    const group = groupOf(row);
    const part = parts.get(row.part);
    const levels = powerLevels(part);
    const quantity = (mode: string) => {
      const values = view?.modes.find((m) => m.id === mode);
      return (eclipse ? values?.eclipse : values?.sunlit)?.parts[row.part] ?? null;
    };
    let last = groups.at(-1);
    if (!last || last.group !== group) {
      last = { group, label: nodeLabel(design, group), rows: [] };
      groups.push(last);
    }
    last.rows.push({
      part: row.part,
      count: part?.count ?? 1,
      kind: row.kind && kindLabel(design.defs, row.kind),
      levels:
        !system || system === "power"
          ? levels.map(([, q]) => q.display).join(" / ")
          : row.states.join(" / "),
      levelsTitle: levels.map(([state, q]) => `${state}: ${q.display}`).join(" / "),
      cells: cellsOf(table, draft, { row, index, levels, quantity }, words, eclipse),
    });
  });
  return groups;
}
