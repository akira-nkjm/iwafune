import type { PowerBudgetSegment, PowerModeView } from "@/api/types";
import { localized, useT } from "@/i18n";
import { cn } from "@/lib/cn";
import { nodeName } from "@/lib/design";
import { systemTone } from "@/lib/systems";
import { toneHex } from "@/lib/tone";
import { quantityText } from "@/quantity/format";

/** 区間の塗り。系の消費は系の色、充電・余り・不足は電源の色を薄めて区別する（幅は API の fraction） */
function segmentStyle(s: PowerBudgetSegment) {
  if (s.kind === "load") return { background: toneHex[systemTone(s.system)] };
  if (s.kind === "charge") return { background: toneHex.green, opacity: 0.4 };
  if (s.kind === "shunt") return { background: toneHex.gray, opacity: 0.35 };
  return { background: toneHex.red, opacity: 0.25 };
}

/**
 * モードごとに、日照中（または蝕中）の電力の行き先を 1 本の積み上げ棒で並べる。目盛りは全モードで共通。
 * 行を押すとそのモードを選ぶ（判定のバーなどがそのモードに切り替わる）。
 */
export function PowerBudget({
  modes,
  eclipse,
  selected,
  onSelect,
}: {
  modes: PowerModeView[];
  eclipse: boolean;
  selected: string | undefined;
  onSelect: (id: string) => void;
}) {
  const t = useT().systems.power;
  const legend = new Map<string, PowerBudgetSegment>();
  return (
    <div className="space-y-1.5">
      {modes.map((m) => {
        const budget = eclipse ? m.eclipse : m.sunlit;
        for (const s of budget?.segments ?? [])
          legend.set(s.kind === "load" ? `load:${s.system}` : s.kind, s);
        return (
          <button
            key={m.id}
            type="button"
            onClick={() => onSelect(m.id)}
            aria-pressed={m.id === selected}
            className={cn(
              "grid w-full grid-cols-[minmax(0,7rem)_minmax(0,1fr)_4.5rem] items-center gap-3 rounded-control px-2 py-1.5 text-left text-xs hover:bg-panel-2",
              m.id === selected && "bg-accent-tint",
            )}
          >
            <span className="truncate font-medium">{nodeName(m)}</span>
            <span className="flex h-4 overflow-hidden rounded-[5px] bg-panel-2">
              {budget?.segments.map((s) => (
                <span
                  key={`${s.kind}:${s.system ?? ""}`}
                  title={`${localized(s.label.label, s.label.label_en)} ${quantityText(s.value)}`}
                  className={cn("h-full", s.kind === "deficit" && "ring-1 ring-neg ring-inset")}
                  style={{ width: `${s.fraction * 100}%`, ...segmentStyle(s) }}
                />
              ))}
            </span>
            <span className="num text-right text-ink-3">
              {budget ? quantityText(budget.supply) : t.unavailable}
            </span>
          </button>
        );
      })}
      <div className="flex flex-wrap gap-x-3 gap-y-1 px-2 pt-2 text-ink-3 text-xs">
        {[...legend.values()].map((s) => (
          <span key={`${s.kind}:${s.system ?? ""}`} className="inline-flex items-center gap-1.5">
            <span
              aria-hidden="true"
              className="inline-block size-2.5 rounded-sm"
              style={segmentStyle(s)}
            />
            {localized(s.label.label, s.label.label_en)}
          </span>
        ))}
      </div>
    </div>
  );
}
