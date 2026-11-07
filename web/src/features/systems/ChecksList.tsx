import type { CheckView } from "@/api/types";
import { VerdictPill } from "@/kit/VerdictPill";

/**
 * 判定の一覧（違反 → 注意 → OK の順）。モードごとなど同じ項目の判定が並ぶので、対象（判定の持ち主の名前）を前に添える。
 * 行のキーは判定の場所（`at`。判定ごとに一意）。
 */
export function ChecksList({ checks }: { checks: CheckView[] }) {
  if (checks.length === 0) return null;
  const order = { violation: 0, warning: 1, ok: 2 } as const;
  const sorted = [...checks].sort((a, b) => order[a.verdict] - order[b.verdict]);
  return (
    <ul className="divide-y divide-line-soft rounded-control border border-line">
      {sorted.map((c) => (
        <li key={c.at} className="flex items-start gap-2 px-3 py-1.5 text-xs">
          <VerdictPill verdict={c.verdict} />
          <span className="min-w-0">
            {c.scope && <span className="mr-1.5 text-ink-3">{c.scope}</span>}
            <span className="font-medium text-ink">{c.label}</span>
            <span className="ml-2 text-ink-2">{c.message}</span>
          </span>
        </li>
      ))}
    </ul>
  );
}
