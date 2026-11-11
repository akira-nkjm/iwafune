import type { Quantity, QuantityView } from "./types";

/**
 * Op に詰める量（型は gen の Quantity）。入力欄が /units/parse で読んだ「書かれたままの数値と単位」をそのまま詰める。
 * 単位の換算はしない（SI への換算は Rust 側）。属性の値は打った文字列のまま service に渡す（`api/ops.ts` の `useAttrEdit`）。
 */
export function quantityOf(q: Pick<QuantityView, "value" | "unit">): Quantity {
  return { value: q.value, unit: q.unit };
}
