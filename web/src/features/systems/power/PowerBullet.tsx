import type { PowerBulletView } from "@/api/types";
import { localized, useT } from "@/i18n";
import { VerdictPill } from "@/kit/status/VerdictPill";

/** 許容帯・設計値・限界は API の表示位置をそのまま使う。 */
export function PowerBullet({ item }: { item: PowerBulletView }) {
  const t = useT().systems.power;
  const title = localized(item.label.label, item.label.label_en);
  return (
    <div className="space-y-2 rounded-control border border-line p-3">
      <div className="flex flex-wrap items-center justify-between gap-2">
        <span className="font-medium text-sm">{title}</span>
        {item.verdict && <VerdictPill verdict={item.verdict} />}
      </div>
      <div className="num flex justify-between gap-2 text-sm">
        <span>
          {t.design}: {item.value?.display ?? t.unavailable}
        </span>
        <span className="text-ink-3">
          {t.limit}: {item.limit.display}
        </span>
      </div>
      <svg
        viewBox="0 0 100 12"
        preserveAspectRatio="none"
        role="img"
        aria-label={title}
        className="h-6 w-full"
      >
        <title>{`${title}: ${item.value?.display ?? t.unavailable}; ${t.limit}: ${item.limit.display}`}</title>
        <line x1={0} x2={100} y1={6} y2={6} stroke="var(--color-panel-2)" strokeWidth={5} />
        <line
          x1={item.band_start}
          x2={item.band_end}
          y1={6}
          y2={6}
          stroke="var(--color-pos)"
          strokeOpacity={0.3}
          strokeWidth={5}
        />
        <line
          x1={item.limit_marker}
          x2={item.limit_marker}
          y1={1}
          y2={11}
          stroke="var(--color-ink-3)"
          strokeWidth={0.4}
          strokeDasharray="1 1"
        />
        {item.marker !== null && (
          <line
            x1={item.marker}
            x2={item.marker}
            y1={0}
            y2={12}
            stroke="var(--color-accent)"
            strokeWidth={0.8}
          />
        )}
      </svg>
      <p className="text-2xs text-ink-3">{t.barLegend}</p>
    </div>
  );
}
