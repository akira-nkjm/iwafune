import type { CommandText, ModeCellText, ModeTable, StateAssign } from "@/api/types";
import { numbersInputText } from "@/quantity/format";

/**
 * 下書きのセル: 基本の状態と、ほかの状態の時間割合の打った文字列（`fune mode set --duty` と同じく、数として読むのは service）。
 */
export type CellText = Pick<ModeCellText, "state" | "duty"> & {
  state: string;
  active?: ModeCellText["active"];
  spare_state?: ModeCellText["spare_state"];
};

/**
 * モード表の編集中の下書き。mode → part → セル（null は「未指定にする」）。
 * 保存と下書きの計算では、変えたセルを `fune mode set|unset` の引数（`CommandText` の mode_set）にまとめ、
 * service が 1 つの SetModeStates にする。
 */
export type ModeDraft = ReadonlyMap<string, ReadonlyMap<string, CellText | null>>;

/** 保存済みのセル → 下書きのセルの形（割合は入力欄の文字列にする） */
export function cellText(cell: StateAssign): CellText {
  const duty = Object.entries(cell.duty ?? {}).map(([s, f]) => [s, numbersInputText([f])] as const);
  return {
    state: cell.state,
    duty: Object.fromEntries(duty),
    active: cell.active == null ? null : numbersInputText([cell.active]),
    spare_state: cell.spare_state ?? null,
  };
}

function savedCell(table: ModeTable, mode: string, part: string): CellText | null {
  const cell = table.rows.find((r) => r.part === part)?.cells[mode];
  return cell ? cellText(cell) : null;
}

export function cellOf(
  table: ModeTable,
  draft: ModeDraft,
  mode: string,
  part: string,
): CellText | null {
  const d = draft.get(mode);
  if (d?.has(part)) return d.get(part) ?? null;
  return savedCell(table, mode, part);
}

function sameCell(a: CellText | null, b: CellText | null): boolean {
  if (!a || !b) return !a && !b;
  if (
    a.state !== b.state ||
    (a.active || null) !== (b.active || null) ||
    (a.spare_state || null) !== (b.spare_state || null)
  )
    return false;
  const ka = Object.keys(a.duty).sort();
  const kb = Object.keys(b.duty).sort();
  return ka.length === kb.length && ka.every((k, i) => k === kb[i] && a.duty[k] === b.duty[k]);
}

/** 1 セルを書き換えた下書き（元と同じに戻したら下書きから消す） */
export function withCell(
  table: ModeTable,
  draft: ModeDraft,
  mode: string,
  part: string,
  cell: CellText | null,
): ModeDraft {
  const next = new Map(draft);
  const m = new Map(next.get(mode) ?? []);
  if (sameCell(savedCell(table, mode, part), cell)) m.delete(part);
  else m.set(part, cell);
  if (m.size === 0) next.delete(mode);
  else next.set(mode, m);
  return next;
}

export function draftCount(draft: ModeDraft): number {
  let n = 0;
  for (const m of draft.values()) n += m.size;
  return n;
}

/**
 * 下書きを `fune mode set|unset` の引数にする: 変えたセルだけ（表の列と行の順）。state が null のセルは未指定に戻す。
 * 下書きが無ければ null。Op にするのは service（`useCommandOps`）
 */
export function draftCommand(table: ModeTable, draft: ModeDraft): CommandText | null {
  const cells: ModeCellText[] = [];
  for (const header of table.modes) {
    const m = draft.get(header.id);
    if (!m) continue;
    for (const row of table.rows) {
      if (!m.has(row.part)) continue;
      const c = m.get(row.part) ?? null;
      cells.push({
        mode: header.id,
        part: row.part,
        state: c?.state ?? null,
        active: c?.active ?? null,
        spare_state: c?.spare_state ?? null,
        eclipse: false,
        duty: { ...(c?.duty ?? {}) },
      });
    }
  }
  return cells.length > 0 ? { command: "mode_set", cells } : null;
}
