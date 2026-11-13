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
        {/* 地: 目盛りの全体、薄い帯: 許容の範囲 */}
        <rect x={0} y={3} width={100} height={6} rx={1} fill="var(--color-panel-2)" />
        <rect
          x={item.band_start}
          y={3}
          width={Math.max(0, item.band_end - item.band_start)}
          height={6}
          fill="var(--color-pos)"
          fillOpacity={0.14}
        />
        {/* 設計値の棒（0 から伸ばす。色は判定） */}
        {item.marker !== null && (
          <rect
            x={Math.min(item.origin, item.marker)}
            y={4.5}
            width={Math.abs(item.marker - item.origin)}
            height={3}
            rx={0.6}
            fill={item.verdict === "violation" ? "var(--color-neg)" : "var(--color-pos)"}
          />
        )}
        {/* 限界・必要値 */}
        <line
          x1={item.limit_marker}
          x2={item.limit_marker}
          y1={0.5}
          y2={11.5}
          stroke="var(--color-ink)"
          strokeWidth={0.5}
          strokeDasharray="1.2 0.8"
          vectorEffect="non-scaling-stroke"
        />
      </svg>
    </div>
  );
}
