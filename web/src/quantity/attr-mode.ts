import type { AttributeDefView, QuantityView, ValueView } from "@/api/types";
import { scalarOf, stateValue } from "./format";

/**
 * 属性の値の編集の仕方。属性定義の `value_type` と `by_state` だけで決める（値の形や画面では決めない）。
 * 量（scalar）だけを欄で直し、ほかの型（ベクトル・範囲・列挙など）と定義の無い属性は表示だけ（CLI か TOML で直す）。
 * 部品・カタログ品の欄（`AttrValueEditor`）とパラメーター追加のダイアログが使う。
 */
export type AttrEditMode = "scalar" | "by_state" | "display";

export function attrEditMode(
  def: Pick<AttributeDefView, "value_type" | "by_state"> | undefined,
): AttrEditMode {
  if (def?.value_type !== "scalar") return "display";
  return def.by_state ? "by_state" : "scalar";
}

/** 状態 1 つの値。状態ごとの属性に、どの状態でも同じ値（スカラー）が書かれていればその値 */
export function cellValue(v: ValueView | null | undefined, state: string): QuantityView | null {
  return stateValue(v, state) ?? scalarOf(v);
}
