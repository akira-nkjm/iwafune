import { useEnumLabel } from "@/api/labels";
import type { Verification } from "@/api/types";
import { useT } from "@/i18n";

/** 確かめる先（バジェット、または解析の ID と判定のキー。書かれていなければ何も出さない） */
export function VerifiedBy({ verify }: { verify: Verification }) {
  const t = useT().requirements;
  const label = useEnumLabel();
  if (verify.budget) return <span>{t.budgetOf(label("budget_kind", verify.budget))}</span>;
  if (!verify.analysis) return <span className="text-ink-4">—</span>;
  return (
    <span className="flex flex-col">
      <span className="font-mono text-[12px]">{verify.analysis}</span>
      {verify.check && <span className="font-mono text-[12px] text-ink-3">{verify.check}</span>}
    </span>
  );
}
