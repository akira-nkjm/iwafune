import type { PowerModeView } from "@/api/types";
import { LightingSwitch } from "@/features/modes/matrix/ModesMatrix";
import { useT } from "@/i18n";
import { Chip } from "@/kit/selection/Chip";
import { verdictTone } from "@/kit/status/verdict-tone";

const dots = { pos: "bg-pos", warn: "bg-warn", neg: "bg-neg" };

/** モードと日照条件の選択。判定は API の値を表示する。 */
export function PowerControls({
  modes,
  selected,
  onSelect,
  eclipse,
  onLighting,
}: {
  modes: PowerModeView[];
  selected: string | undefined;
  onSelect: (id: string) => void;
  eclipse?: boolean;
  onLighting?: (value: boolean) => void;
}) {
  const t = useT();
  return (
    <div className="flex flex-wrap items-center justify-between gap-3">
      <fieldset className="flex flex-wrap gap-2" aria-label={t.systems.power.mode}>
        {modes.map((m) => (
          <Chip
            key={m.id}
            mono={false}
            selected={m.id === selected}
            onSelect={() => onSelect(m.id)}
          >
            <span className="inline-flex items-center gap-2">
              <span
                className={`size-1.5 rounded-full ${m.verdict ? dots[verdictTone[m.verdict]] : "bg-ink-4"}`}
              />
              {m.label || m.id}
              <span className="sr-only">
                {m.verdict ? t.verdict[m.verdict] : t.systems.power.unavailable}
              </span>
            </span>
          </Chip>
        ))}
      </fieldset>
      {onLighting && <LightingSwitch value={eclipse ?? false} onChange={onLighting} />}
    </div>
  );
}
