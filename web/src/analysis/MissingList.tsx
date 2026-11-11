import { Link } from "react-router";
import type { DesignView, MissingInput } from "@/api/types";
import { useT } from "@/i18n";
import { targetLink } from "@/lib/target-link";

/** 足りない入力（名指し）。対象の構造から、部品はコンポーネントのページへ、設定はプロジェクトの設定へ飛べる */
export function MissingList({
  design,
  missing,
}: {
  design: DesignView | undefined;
  missing: MissingInput[];
}) {
  const t = useT();
  if (missing.length === 0) return null;
  const partIds = new Set(design?.parts.map((p) => p.id) ?? []);
  return (
    <div className="rounded-control border border-warn/60 bg-warn-bg px-3 py-2">
      <p className="mb-1 font-medium text-warn-ink text-xs">{t.systems.missing(missing.length)}</p>
      <ul className="space-y-0.5 text-xs">
        {missing.map((m) => {
          const to = targetLink(m.target, partIds);
          return (
            <li key={`${m.path}-${m.reason}`} className="flex gap-2">
              {to ? (
                <Link to={to} className="font-mono text-accent-ink hover:underline">
                  {m.path}
                </Link>
              ) : (
                <span className="font-mono text-ink-2">{m.path}</span>
              )}
              <span className="text-ink-2">{m.reason}</span>
            </li>
          );
        })}
      </ul>
    </div>
  );
}
