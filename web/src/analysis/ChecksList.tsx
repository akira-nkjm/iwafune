import type { CheckView } from "@/api/types";
import { localized, useT } from "@/i18n";
import { VerdictPill } from "@/kit/status/VerdictPill";

/**
 * 判定の一覧（違反 → 注意 → OK の順）。モードごとなど同じ項目の判定が並ぶので、対象（判定の持ち主の名前）を前に添える。
 * 行のキーは判定の場所（`at`。判定ごとに一意）。語彙から引いた判定（期間不足など）は英語の文も持つ。
 * 持ち主ごとにまとめて見出しを付けたところ（概要の解析の表）では `showScope={false}` で対象を省く。
 */
export function ChecksList({
  checks,
  showScope = true,
}: {
  checks: CheckView[];
  showScope?: boolean;
}) {
  // 言語を切り替えたら描き直す（`localized` は今の言語で選ぶ）
  useT();
  if (checks.length === 0) return null;
  const order = { violation: 0, warning: 1, ok: 2 } as const;
  const sorted = [...checks].sort((a, b) => order[a.verdict] - order[b.verdict]);
  return (
    <ul className="divide-y divide-line-soft rounded-control border border-line">
      {sorted.map((c) => (
        <li key={c.at} className="flex items-start gap-2 px-3 py-1.5 text-xs">
          <VerdictPill verdict={c.verdict} />
          <span className="min-w-0">
            {showScope && c.scope && <span className="mr-1.5 text-ink-3">{c.scope}</span>}
            <span className="font-medium text-ink">{localized(c.label, c.label_en)}</span>
            <span className="ml-2 text-ink-2">{localized(c.message, c.message_en)}</span>
          </span>
        </li>
      ))}
    </ul>
  );
}
