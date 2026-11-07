import { useT } from "@/i18n";
import type { PowerCategory } from "./power";

/** 右下の凡例: 電源の系統ごとの線の色と、信号・RF */
export function DiagramLegend({
  categories,
  signal,
}: {
  categories: PowerCategory[];
  signal: boolean;
}) {
  const t = useT().diagram.legend;
  if (categories.length === 0 && !signal) return null;
  return (
    <div className="absolute right-3 bottom-3 z-10 min-w-48 rounded-card border border-line bg-panel/95 px-3 py-2 text-[11px] shadow-pop">
      <div className="mb-1 font-semibold text-ink-2">{t.title}</div>
      <ul className="space-y-0.5">
        {categories.map((c) => (
          <li key={c.key} className="flex items-center gap-2">
            <span className="inline-block h-0.5 w-6" style={{ background: c.color }} />
            {c.label}
          </li>
        ))}
        {signal && (
          <>
            <li className="flex items-center gap-2">
              <span className="inline-block h-0.5 w-6 bg-cat-blue" />
              {t.signal}
            </li>
            <li className="flex items-center gap-2">
              <span className="inline-block h-0 w-6 border-cat-rose border-t-2 border-dashed" />
              {t.rf}
            </li>
          </>
        )}
      </ul>
    </div>
  );
}
