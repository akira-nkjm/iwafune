import type { AddChoices, CommandText, DesignView } from "@/api/types";
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
