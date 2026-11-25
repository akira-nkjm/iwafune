import type { AddChoices, CommandText, DesignView, LinkView, Op, SetText } from "@/api/types";
import type { SelectOption } from "@/kit/forms/NativeSelect";
import { idWithName } from "@/lib/design";

/** 回線を足す欄（打った文字列のまま。向きの綴りと部品・帯域の実在は service の `link_add_op` と適用のときの検査が見る） */
export interface NewLink {
  id: string;
  direction: string;
  part: string;
  band: string;
}

/** 空の欄（足りない入力が回線の ID を名指ししていれば、`initialId` で始める） */
export function emptyLink(initialId?: string | null): NewLink {
  return { id: initialId ?? "", direction: "", part: "", band: "" };
}

/**
 * `fune link add` と同じ引数（`POST /ops/from_command` の `link_add`）。4 つの欄が埋まるまでは作らない
 * （読み込める回線は向き・部品・帯域が揃ったものだけ）。
 */
export function linkAddCommand(l: NewLink): CommandText | null {
  if (!l.id.trim() || !l.direction || !l.part || !l.band) return null;
  return { command: "link_add", ...l };
}

/** 回線の衛星側にできる部品の選択肢（API の `add_choices.transceivers`。表示は製品ツリーの名前を添える） */
export function transceiverOptions(design: DesignView): SelectOption[] {
  return design.add_choices.transceivers.map((id) => {
    const node = design.tree.find((n) => n.id === id);
    return { value: id, label: node ? idWithName(node) : id };
  });
}

/** 地上局側の帯域の選択肢（API の `add_choices.bands`） */
export function bandOptions(choices: AddChoices): SelectOption[] {
  return choices.bands.map((b) => ({ value: b, label: b }));
}

/** 開いて直せる回線の欄（`fune set <回線の設定のパス>.<欄> <値>` で 1 つずつ書く。ID は名前の変更 `RenameCommLink`） */
export const LINK_FIELDS = ["direction", "part", "band"] as const;
export type LinkField = (typeof LINK_FIELDS)[number];

/** 今の回線を欄の形にする（開いて直すダイアログの初期値） */
export function linkFields(view: LinkView): NewLink {
  return { id: view.id, direction: view.direction, part: view.part, band: view.band };
}

/**
 * 直した欄 1 つを書く文字列（`POST /ops/from_text`。パスは API が返した回線の設定のパス + 欄の名前）。
 * 変えていない欄と、空にした欄は送らない（null）
 */
export function linkFieldText(view: LinkView, next: NewLink, field: LinkField): SetText | null {
  const value = next[field].trim();
  if (!value || value === view[field]) return null;
  return {
    item: null,
    path: `${view.group}.${field}`,
    value,
    source: null,
    at: null,
    note: null,
    min: null,
    max: null,
    ops: [],
  };
}

/** ID を変えたときの新しい ID（変えていない・空なら null） */
export function linkRenamedTo(view: LinkView, next: NewLink): string | null {
  const to = next.id.trim();
  return to && to !== view.id ? to : null;
}

/**
 * 回線を直す Op の列。欄の書き換え（`fieldOps` は `LINK_FIELDS` の順に、直した欄の Op か、直していない・まだ Op になっていない null）
 * のあとに名前の変更を置く（欄は今の名前で書き、最後に名前を変える）。直した欄の Op がまだ届いていないか、何も直していなければ null
 */
export function linkEditOps(
  view: LinkView,
  next: NewLink,
  fieldOps: readonly (Op | null)[],
): Op[] | null {
  const ops: Op[] = [];
  for (const [i, field] of LINK_FIELDS.entries()) {
    if (!linkFieldText(view, next, field)) continue;
    const op = fieldOps[i];
    if (!op) return null;
    ops.push(op);
  }
  const to = linkRenamedTo(view, next);
  if (to) ops.push({ op: "rename_comm_link", id: view.id, to });
  return ops.length > 0 ? ops : null;
}

/** 選択肢に今の値が無ければ（部品が送受信機でなくなった・帯域の性能を消したなど）先頭に足す（開いた回線の値を黙って変えない） */
export function withCurrent(options: SelectOption[], current: string | null): SelectOption[] {
  if (!current || options.some((o) => o.value === current)) return options;
  return [{ value: current, label: current }, ...options];
}
