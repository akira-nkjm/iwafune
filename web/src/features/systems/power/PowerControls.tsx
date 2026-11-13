import type { PowerModeView } from "@/api/types";
import { LightingSwitch } from "@/features/modes/matrix/ModesMatrix";
import { useT } from "@/i18n";
import { NativeSelect } from "@/kit/forms/NativeSelect";
import { Chip } from "@/kit/selection/Chip";
import { verdictTone } from "@/kit/status/verdict-tone";
import { nodeName } from "@/lib/design";

const dots = { pos: "bg-pos", warn: "bg-warn", neg: "bg-neg" };

/** これより多いモードはチップでなく選択の欄にする（チップが何行にも折り返さないように） */
const MAX_CHIPS = 6;

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
  const verdictText = (m: PowerModeView) =>
    m.verdict ? t.verdict[m.verdict] : t.systems.power.unavailable;
  const dot = (m: PowerModeView | undefined) => (
    <span
      aria-hidden="true"
      className={`size-1.5 shrink-0 rounded-full ${m?.verdict ? dots[verdictTone[m.verdict]] : "bg-ink-4"}`}
    />
  );
  return (
    <div className="flex flex-wrap items-center justify-between gap-3">
      {modes.length > MAX_CHIPS ? (
        <div className="inline-flex items-center gap-2">
          {dot(modes.find((m) => m.id === selected))}
          <NativeSelect
            aria-label={t.systems.power.mode}
            value={selected ?? ""}
            onChange={(e) => onSelect(e.target.value)}
            options={modes.map((m) => ({
              value: m.id,
              label: `${nodeName(m)} — ${verdictText(m)}`,
            }))}
          />
        </div>
      ) : (
        <fieldset className="flex flex-wrap gap-2" aria-label={t.systems.power.mode}>
          {modes.map((m) => (
            <Chip
              key={m.id}
              mono={false}
              selected={m.id === selected}
              onSelect={() => onSelect(m.id)}
            >
              <span className="inline-flex items-center gap-2">
                {dot(m)}
                {nodeName(m)}
                <span className="sr-only">{verdictText(m)}</span>
              </span>
            </Chip>
          ))}
        </fieldset>
      )}
      {onLighting && <LightingSwitch value={eclipse ?? false} onChange={onLighting} />}
    </div>
  );
}
