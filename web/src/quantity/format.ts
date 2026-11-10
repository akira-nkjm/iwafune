import type { QuantityView, ValueView, Vector3 } from "@/api/types";

/**
 * 値の表示文字列。**数値の整形と単位の換算はしない**（service が返す display をそのまま使う）。
 * ここは「どの display を、どう並べるか」だけを決める。
 */
export function quantityText(q: QuantityView | null | undefined): string {
  return q ? q.display : "—";
}

/** 値 1 つを 1 行の文字列にする（表のセル向け）。文字は service の display（整形は Rust）をそのまま使う */
export function valueText(v: ValueView | null | undefined): string {
  if (!v || v.value.type === "unset") return "—";
  return v.display ?? "—";
}

/** 入力欄の初期値（書かれたままの数値と単位。換算しない） */
export function quantityInputText(q: QuantityView | null | undefined): string {
  if (!q) return "";
  return q.unit ? `${q.value} ${q.unit}` : String(q.value);
}

/** 数の並びの入力欄の初期値（`"1 0 0 0"`。API の数をそのまま並べる。読むのは service） */
export function numbersInputText(values: readonly number[]): string {
  return values.map(String).join(" ");
}

/** ベクトルの入力欄の初期値（`"0 0 12 mm"`。書かれたままの数値と単位。換算しない。読むのは service） */
export function vectorInputText(v: Pick<Vector3, "x" | "y" | "z" | "unit">): string {
  const xyz = numbersInputText([v.x, v.y, v.z]);
  return v.unit ? `${xyz} ${v.unit}` : xyz;
}

/** 状態ごとの値から 1 つの状態を引く */
export function stateValue(v: ValueView | null | undefined, state: string): QuantityView | null {
  if (v?.value.type !== "by_state") return null;
  return v.value.states[state] ?? null;
}

/** スカラーの値（nominal）。スカラーでなければ null */
export function scalarOf(v: ValueView | null | undefined): QuantityView | null {
  return v && v.value.type === "scalar" ? v.value.nominal : null;
}
