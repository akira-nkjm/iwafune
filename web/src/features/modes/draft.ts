import type { ModeCell, ModeTable, Op, StateAssign } from "@/api/types";

/**
 * モード表の編集中の下書き。mode → part → セル（null は「未指定にする」）。
 * 保存のときにモードごとの SetModeStates（そのモードの全行）にまとめる。
 */
export type ModeDraft = ReadonlyMap<string, ReadonlyMap<string, StateAssign | null>>;

export function cellOf(
  table: ModeTable,
  draft: ModeDraft,
  mode: string,
  part: string,
): StateAssign | null {
  const d = draft.get(mode);
  if (d?.has(part)) return d.get(part) ?? null;
  return table.rows.find((r) => r.part === part)?.cells[mode] ?? null;
}

function sameCell(a: StateAssign | null | undefined, b: StateAssign | null | undefined): boolean {
  if (!a || !b) return !a && !b;
  if (a.state !== b.state) return false;
  const da = a.duty ?? {};
  const db = b.duty ?? {};
  const ka = Object.keys(da).sort();
  const kb = Object.keys(db).sort();
  return ka.length === kb.length && ka.every((k, i) => k === kb[i] && da[k] === db[k]);
}

/** 1 セルを書き換えた下書き（元と同じに戻したら下書きから消す） */
export function withCell(
  table: ModeTable,
  draft: ModeDraft,
  mode: string,
  part: string,
  cell: StateAssign | null,
): ModeDraft {
  const original = table.rows.find((r) => r.part === part)?.cells[mode] ?? null;
  const next = new Map(draft);
  const m = new Map(next.get(mode) ?? []);
  if (sameCell(original, cell)) m.delete(part);
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

/** 下書きを Op にする: 変えたセルだけを 1 つの SetModeStates（cells）にまとめる。state が null のセルは未指定に戻す */
export function draftOps(table: ModeTable, draft: ModeDraft): Op[] {
  const cells: ModeCell[] = [];
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
        duty: { ...(c?.duty ?? {}) },
      });
    }
  }
  return cells.length > 0 ? [{ op: "set_mode_states", cells }] : [];
}
