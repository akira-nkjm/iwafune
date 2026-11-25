import type { DesignView } from "@/api/types";
import { localized } from "@/i18n";
import { kindLabel } from "@/lib/design";

/** 部品を足すときに選べるカタログの品（ID・表示名・補足） */
export interface PickOption {
  id: string;
  /** 表示名（無ければ空） */
  label: string;
  /** 種別・製造元（無ければ空） */
  note: string;
}

/**
 * 選べる品の一覧（ID 順）。カタログの品（共有カタログを含む）と、部品がすでに参照している品
 * （カタログに載っていないものも選べるようにする）
 */
export function pickOptions(design: DesignView): PickOption[] {
  const byId = new Map<string, PickOption>();
  for (const c of design.catalog) {
    const note = [kindLabel(design.defs, c.kind), c.manufacturer].filter(Boolean).join(" · ");
    byId.set(c.id, { id: c.id, label: localized(c.label, c.label_en), note });
  }
  for (const p of design.parts) {
    for (const id of [...p.candidates, ...(p.catalog ? [p.catalog] : [])]) {
      if (!byId.has(id)) byId.set(id, { id, label: "", note: "" });
    }
  }
  return [...byId.values()].sort((a, b) => a.id.localeCompare(b.id));
}

/** 打った文字で絞る（ID・表示名・補足のどれかに、大文字小文字を区別せず含まれる。空白区切りはすべて含む） */
export function filterPickOptions(options: readonly PickOption[], query: string): PickOption[] {
  const words = query.toLowerCase().split(/\s+/).filter(Boolean);
  if (words.length === 0) return [...options];
  return options.filter((o) => {
    const hay = `${o.id} ${o.label} ${o.note}`.toLowerCase();
    return words.every((w) => hay.includes(w));
  });
}

/** カタログの品の系に当たる置き場（グループの選択肢にその系があるときだけ。無ければ null） */
export function groupForSystem(
  groups: readonly { value: string }[],
  system: string | null | undefined,
): string | null {
  return system && groups.some((g) => g.value === system) ? system : null;
}
