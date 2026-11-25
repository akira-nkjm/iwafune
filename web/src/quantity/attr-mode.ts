import type { AttrInput, AttributeDefView, QuantityView, ValueView } from "@/api/types";
import { scalarOf, stateValue } from "./format";

/**
 * 属性の値の入力の形。**決めるのは API（`AttributeDefView.input`。service の `fixes::attr_input`）** で、ここは取り出すだけ
 * （値の型や属性の名前では分けない）。定義の無い属性は null（表示だけ）。
 * 部品・カタログ品の欄（`AttrValueEditor`）とパラメーター追加のダイアログが使う。
 */
export function attrInput(def: Pick<AttributeDefView, "input"> | undefined): AttrInput | null {
  return def?.input ?? null;
}

/** 欄で量として直せるか（パラメーター追加のダイアログは量の欄だけを持つ） */
export function isQuantityInput(def: Pick<AttributeDefView, "input"> | undefined): boolean {
  return attrInput(def)?.kind === "quantity";
}

/** 状態 1 つの値。状態ごとの属性に、どの状態でも同じ値（スカラー）が書かれていればその値 */
export function cellValue(v: ValueView | null | undefined, state: string): QuantityView | null {
  return stateValue(v, state) ?? scalarOf(v);
}

/** 選んで入れる・打って入れる欄の今の値の綴り（列挙・文字列・真偽。それ以外と未設定は空） */
export function choiceText(v: ValueView | null | undefined): string {
  const d = v?.value;
  if (!d) return "";
  if (d.type === "enum" || d.type === "text") return d.value;
  if (d.type === "bool") return String(d.value);
  return "";
}
