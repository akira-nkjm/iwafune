import { MissingList } from "@/analysis/MissingList";
import type { BudgetPropellant, BudgetTotal, DesignView, QuantityView } from "@/api/types";
import { useT } from "@/i18n";
import { TickBar } from "@/kit/metrics/TickBar";
import { VerdictPill } from "@/kit/status/VerdictPill";
import { wetTotal } from "./cards";

/**
 * 質量のバジェットの合計: 乾燥と推薬込みを 1 行ずつ、それぞれの余裕と判定（API の値）を添えて並べる。
 * 推薬を数えなかったときは乾燥の行だけ。
 */
export function MassTotals({
  dry,
  dryVerdict,
  dryMargin,
  propellant,
}: {
  dry: QuantityView;
  dryVerdict: BudgetTotal["verdict"];
  dryMargin: QuantityView | null;
  propellant: BudgetPropellant;
}) {
  const t = useT().overview;
  const wet = wetTotal(propellant);
  const line = (
    label: string,
    total: QuantityView,
    v: BudgetTotal["verdict"],
    m: QuantityView | null,
  ) => (
    <div className="flex items-baseline gap-2">
      <span className="w-[110px] text-ink-3 text-xs">{label}</span>
      <span className="num font-semibold text-[16px]">{total.display}</span>
      {v && <VerdictPill verdict={v} text={m ? t.budgetMargin(m.display) : undefined} />}
    </div>
  );
  return (
    <div className="space-y-1">
      {line(t.budgetDry, dry, dryVerdict, dryMargin)}
      {wet && line(t.budgetWet, wet.total, wet.verdict, wet.margin)}
    </div>
  );
}

/** 推薬の行（配分の行と同じ並び。推薬は配分を持たないので目盛りは「データなし」）。数えなかったときは理由 */
export function PropellantRow({
  propellant: p,
  design,
}: {
  propellant: BudgetPropellant;
  design: DesignView | undefined;
}) {
  const t = useT().overview;
  if (p.status === "counted")
    return (
      <li className="grid grid-cols-[110px_minmax(0,1fr)_auto] items-center gap-3 text-xs">
        <span className="truncate font-mono" title={p.analysis}>
          {t.budgetPropellant}
        </span>
        <TickBar ticks={36} ratio={null} label={`${t.budgetPropellant}: ${p.loaded.display}`} />
        <span className="num whitespace-nowrap text-right text-ink-2">{p.loaded.display}</span>
      </li>
    );
  return (
    <li className="space-y-1 text-[11px] text-ink-3">
      <p>{omittedText(t, p)}</p>
      {p.status === "missing" && <MissingList design={design} missing={p.missing} />}
    </li>
  );
}

/** 推薬を数えなかった理由の文言（理由は API の構造） */
function omittedText(
  t: ReturnType<typeof useT>["overview"],
  p: Exclude<BudgetPropellant, { status: "counted" }>,
): string {
  switch (p.status) {
    case "no_propulsion":
      return t.propellantNone;
    case "missing":
      return t.propellantMissing;
    case "failed":
      return t.propellantFailed(p.message);
  }
}
