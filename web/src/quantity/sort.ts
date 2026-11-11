import type { ValueData } from "@/api/types";

/** 表の比較に使う SI 値。単体は nominal、範囲は min、状態別は最大値。 */
export function valueSortSi(value: ValueData | null | undefined): number | null {
  switch (value?.type) {
    case "scalar":
      return value.nominal.si;
    case "range":
      return value.min.si;
    case "by_state": {
      const values = Object.values(value.states).map((q) => q.si);
      return values.length ? Math.max(...values) : null;
    }
    default:
      return null;
  }
}
