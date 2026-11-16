import type { BudgetUncounted } from "@/api/types";
import { useT } from "@/i18n";

/**
 * 値が無くて合計に数えなかった部品（API の `BudgetView.uncounted`）。0 とはみなしていないことを 1 行で知らせる。
 * 電力は数えなかったモードを `[…]` で添える。空なら何も出さない。
 */
export function BudgetUncountedNote({ uncounted }: { uncounted: BudgetUncounted[] }) {
  const t = useT().overview;
  if (uncounted.length === 0) return null;
  const names = uncounted
    .map((u) => (u.modes.length > 0 ? `${u.label} [${u.modes.join(", ")}]` : u.label))
    .join(", ");
  return (
    <p className="mt-3 text-[11px] text-ink-3">{t.budgetUncounted(uncounted.length, names)}</p>
  );
}
