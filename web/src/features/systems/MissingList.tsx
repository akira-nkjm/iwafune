import { Link } from "react-router";
import type { DesignView, MissingInput } from "@/api/types";
import { useT } from "@/i18n";

/** 足りない入力（名指し）。部品のパスはコンポーネントのページへ、設定はプロジェクトの設定へ飛べる */
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
          const head = m.part ?? "";
          const to = partIds.has(head) ? `/design/components/${head}` : "/project/settings";
          return (
            <li key={m.path} className="flex gap-2">
              <Link to={to} className="font-mono text-accent-ink hover:underline">
                {m.path}
              </Link>
              <span className="text-ink-2">{m.reason}</span>
            </li>
          );
        })}
      </ul>
    </div>
  );
}
