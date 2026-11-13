import type { PowerVerdictView } from "@/api/types";
import { localized, useT } from "@/i18n";
import { VerdictPill } from "@/kit/status/VerdictPill";

/**
 * 判定 1 つ。値と限界、その比（または不足・余裕）を文字で出し、メーターは「限界に対する割合」を 1 本で描く。
 * 位置（塗り・限界の線）は API の値をそのまま使う。`modeName` は最悪をとったモードの表示名
 */
export function PowerVerdictCard({
  item,
  modeName,
}: {
  item: PowerVerdictView;
  modeName?: string;
}) {
  const t = useT().systems.power;
  const title = localized(item.label.label, item.label.label_en);
  const note = localized(item.note.label, item.note.label_en);
  const limitWord = item.kind === "at_most" ? t.upperLimit : t.required;
  return (
    <div className="rounded-control border border-line p-3">
      <div className="flex items-center justify-between gap-2">
        <span className="font-semibold text-sm">{title}</span>
        {item.verdict && <VerdictPill verdict={item.verdict} />}
      </div>
      <div className="mt-1.5 flex items-baseline gap-2">
        <span className="num font-semibold text-xl">{item.value?.display ?? t.unavailable}</span>
        <span className="num text-ink-3 text-xs">
          / {limitWord} {item.limit.display}
        </span>
      </div>
      <div
        role="img"
        aria-label={`${title}: ${note}`}
        className="relative mt-3 mb-1.5 h-2 rounded-full bg-panel-2"
      >
        <span
          className={`absolute inset-y-0 rounded-full ${item.verdict === "violation" ? "bg-neg" : "bg-pos"}`}
          style={{ left: `${item.fill_start}%`, width: `${item.fill_end - item.fill_start}%` }}
        />
        <span
          aria-hidden="true"
          className="absolute -inset-y-1 w-0.5 rounded-full bg-ink"
          style={{ left: `calc(${item.limit_at}% - 1px)` }}
        />
      </div>
      <div className="flex justify-between gap-2 text-ink-3 text-xs">
        <span>{note}</span>
        {modeName && <span>{t.worstMode(modeName)}</span>}
      </div>
    </div>
  );
}
