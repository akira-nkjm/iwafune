import type { DefsView, System } from "@/api/types";
import { useT } from "@/i18n";
import { Toggle } from "@/kit/selection/Toggle";
import { systemTitle } from "@/lib/design";

/** API の系の一覧と順序に従う表示トグル。 */
export function DiagramSystems({
  defs,
  hidden,
  onChange,
}: {
  defs: DefsView;
  hidden: ReadonlySet<System>;
  onChange: (hidden: ReadonlySet<System>) => void;
}) {
  const t = useT().diagram.toolbar;
  return (
    <fieldset className="flex flex-wrap items-center gap-1.5" aria-label={t.systems}>
      <span className="mr-0.5 text-[11px] text-ink-3">{t.systems}</span>
      {defs.systems.map((system) => (
        <Toggle
          key={system}
          value={!hidden.has(system)}
          onChange={(show) => {
            const next = new Set(hidden);
            if (show) next.delete(system);
            else next.add(system);
            onChange(next);
          }}
        >
          {systemTitle(system, defs)}
        </Toggle>
      ))}
      <span className="text-[11px] text-ink-3">{t.hiddenConnections}</span>
    </fieldset>
  );
}
