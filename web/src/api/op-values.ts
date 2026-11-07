import type { AttrValue, Quantity, QuantityView } from "./types";

/**
 * Op の値の組み立て（型は gen の Op）。入力欄が /units/parse で読んだ「書かれたままの数値と単位」をそのまま詰める。
 * 単位の換算はしない（SI への換算は Rust 側）。
 */
export function scalarValue(q: Pick<QuantityView, "value" | "unit">): AttrValue {
  return { type: "scalar", data: { value: q.value, unit: q.unit } };
}

export function quantityOf(q: Pick<QuantityView, "value" | "unit">): Quantity {
  return { value: q.value, unit: q.unit };
}

/** パラメーター 1 つ（状態ごとの値なら state を付ける）に値を書く Op */
export function setValueOp(
  part: string,
  attr: string,
  state: string | null,
  q: Pick<QuantityView, "value" | "unit">,
) {
  return { op: "set_value" as const, part, attr, state, value: scalarValue(q), from: null };
}

/** 上書きを消してカタログの値に戻す Op */
export function unsetValueOp(part: string, attr: string, state: string | null) {
  return { op: "unset_value" as const, part, attr, state };
}
