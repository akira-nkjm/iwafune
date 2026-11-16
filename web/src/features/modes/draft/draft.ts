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
 * モード表の編集中の下書き。日照／蝕ごとのキー → part → セル（null は日照では未指定、蝕では上書き解除）。
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

/** 日照と蝕を同じ下書きで保持する。NUL はモード ID に含まれない。 */
export function draftKey(mode: string, eclipse = false): string {
  return eclipse ? `\0eclipse:${mode}` : mode;
}

function savedCell(table: ModeTable, mode: string, part: string, eclipse = false): CellText | null {
  const row = table.rows.find((r) => r.part === part);
  const cell = eclipse ? row?.eclipse_cells[mode] : row?.cells[mode];
  return cell ? cellText(cell) : null;
}

export function cellOf(
  table: ModeTable,
  draft: ModeDraft,
  mode: string,
  part: string,
  eclipse = false,
): CellText | null {
  const d = draft.get(draftKey(mode, eclipse));
  if (d?.has(part)) return d.get(part) ?? null;
  return savedCell(table, mode, part, eclipse);
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
  eclipse = false,
): ModeDraft {
  const next = new Map(draft);
  const key = draftKey(mode, eclipse);
  const m = new Map(next.get(key) ?? []);
  if (sameCell(savedCell(table, mode, part, eclipse), cell)) m.delete(part);
  else m.set(part, cell);
  if (m.size === 0) next.delete(key);
  else next.set(key, m);
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
  for (const eclipse of [false, true]) {
    for (const header of table.modes) {
      const m = draft.get(draftKey(header.id, eclipse));
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
          eclipse,
          duty: { ...(c?.duty ?? {}) },
        });
      }
    }
  }
  return cells.length > 0 ? { command: "mode_set", cells } : null;
}
